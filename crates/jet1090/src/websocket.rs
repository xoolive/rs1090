use std::{
    collections::{hash_map::Entry, HashMap},
    fmt::{self, Display},
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use evalexpr::{
    build_operator_tree, ContextWithMutableVariables, HashMapContext, Value,
};
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use rs1090::decode::{TimeSource, TimedMessage, DF};
use serde::{Deserialize, Serialize};
// use serde_tuple::{Deserialize_tuple, Serialize_tuple};
use tokio::{
    sync::{broadcast, Mutex},
    task::JoinHandle,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{debug, error, info};
use uuid::Uuid;
use warp::filters::ws::{Message, WebSocket};

pub struct ChannelState {
    pub ctl: Mutex<ChannelControl>,
}

/// reply data structures
#[derive(Clone, Debug, Serialize)]
pub struct ReplyMessage {
    pub join_reference: Option<String>, // null when it's heartbeat
    pub reference: String,
    pub topic: String, // `channel`
    pub event: String,
    pub payload: ReplyPayload,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReplyPayload {
    pub status: String,
    pub response: Response,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum Response {
    Empty {},
    //Join {},
    //Heartbeat {},
    //Datetime { datetime: String, counter: u32 },
    Jet1090 { timed_message: Box<TimedMessage> },
}

/// RequestMessage is a message from client through websocket
/// it's deserialized from a JSON array
#[derive(Debug, Deserialize)]
struct RequestMessage {
    join_reference: Option<String>, // null when it's heartbeat
    reference: String,
    topic: String, // `channel`
    event: String,
    //payload: RequestPayload,
}

/*#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RequestPayload {
    Join {},
    Leave {},
    Heartbeat {},
}
*/

#[derive(Clone, Debug, Serialize)]
pub enum ChannelMessage {
    Reply(Box<ReplyMessage>),
    ReloadFilter { agent_id: String, code: String },
}

/// agent channel, can broadcast to every agent in the channel
struct Channel {
    /// channel name
    pub name: String,
    /// broadcast in channels
    sender: broadcast::Sender<ChannelMessage>,
    /// channel agents
    agents: Mutex<Vec<String>>,
    /// channel agent count
    count: AtomicU32,
}

struct ChannelAgent {
    channel_name: String,
    join_task: JoinHandle<()>,
}

impl Display for ChannelAgent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "<ChannelAgent: channel={}, task={:?}>",
            self.channel_name, self.join_task
        )
    }
}

/// manages all channels
pub struct ChannelControl {
    channel_map: Mutex<HashMap<String, Channel>>, // channel name -> Channel

    /// agent_id -> Vec<agentTask>
    /// task forwarding channel messages to agent websocket tx
    /// created when agent joins a channel
    agent_task_map: Mutex<HashMap<String, Vec<ChannelAgent>>>,

    conn_sender_map: Mutex<HashMap<String, broadcast::Sender<ChannelMessage>>>, // conn_id -> Sender
    agent_sender_map: Mutex<HashMap<String, broadcast::Sender<ChannelMessage>>>, // agent_id -> Sender
}

#[derive(Debug)]
pub enum ChannelError {
    /// channel does not exist
    ChannelNotFound,
    /// can not send message to channel
    MessageSendError,
    /// you have not called init_agent
    AgentNotInitiated,
}

pub async fn on_ws_connected(ws: WebSocket, state: Arc<ChannelState>) {
    let connect_id = Uuid::new_v4().to_string();
    info!("on_ws_connected: {}", connect_id);

    state
        .ctl
        .lock()
        .await
        .add_connection(connect_id.clone())
        .await;

    // Split WebSocket stream into sender and receiver
    let (ws_tx, ws_rx) = ws.split();

    let mut ws_tx_task = tokio::spawn(websocket_sender(
        connect_id.clone(),
        state.clone(),
        ws_tx,
    ));

    let mut ws_rx_task =
        tokio::spawn(handle_events(ws_rx, state.clone(), connect_id.clone()));

    tokio::select! {
        _ = (&mut ws_tx_task) => ws_rx_task.abort(),
        _ = (&mut ws_rx_task) => ws_tx_task.abort(),
    }

    state
        .ctl
        .lock()
        .await
        .remove_agent(connect_id.to_string())
        .await;

    error!("client connection closed");

    // Create a channel to send data to the WebSocket
    //let (tx, mut rx) = mpsc::unbounded_channel();

    // Spawn a task to forward messages from the channel to the WebSocket
    /*tokio::task::spawn(async move {
        while let Some(message) = rx.recv().await {
            if ws_tx.send(Message::text(message)).await.is_err() {
                break;
            }
        }
    });*/

    // Receive and handle messages from the WebSocket client
    /*  while let Some(Ok(message)) = ws_rx.next().await {
        if let Ok(text) = message.to_str() {
            println!("Received message from client: {}", text);
            tx.send("Hello from server!".to_string()).unwrap();
        }
    }*/
}

pub async fn jet1090_data_task(
    local_state: Arc<ChannelState>,
    mut data_source: UnboundedReceiverStream<TimedMessage>,
    channel_name: &str,
    event_name: &str,
) {
    info!("launch jet1090/data task ...");
    let mut counter = 0;
    while let Some(timed_message) = data_source.next().await {
        let reply_message = ReplyMessage {
            join_reference: None,
            reference: counter.to_string(),
            topic: channel_name.to_string(),
            event: event_name.to_string(),
            payload: ReplyPayload {
                status: "ok".to_string(),
                response: Response::Jet1090 {
                    timed_message: Box::new(timed_message),
                },
            },
        };
        // unexpected error: Error("can only flatten structs and maps (got a integer)", line: 0, column: 0)
        let serialized_result = serde_json::to_string(&reply_message);
        if serialized_result.is_err() {
            error!("error: {}", serialized_result.err().unwrap());
            continue;
        }
        let text = serialized_result.unwrap();
        match local_state
            .ctl
            .lock()
            .await
            .broadcast(
                channel_name.to_string(),
                ChannelMessage::Reply(Box::new(reply_message)),
            )
            .await
        {
            Ok(_) => {
                counter += 1;
                debug!("{} > {}", event_name, text);
            }
            Err(e) => {
                error!(
                    "fail to send, channel: {}, event: {}, err: {:?}",
                    channel_name, event_name, e
                );
            }
        }
    }
}

impl Channel {
    pub fn new(name: String, capacity: Option<usize>) -> Channel {
        let (tx, _rx) = broadcast::channel(capacity.unwrap_or(100));
        Channel {
            name,
            sender: tx,
            agents: Mutex::new(vec![]),
            count: AtomicU32::new(0),
        }
    }

    /// agent joins the channel, returns a sender to the channel
    /// if agent does not exist, a new agent is added
    pub async fn join(
        &self,
        agent_id: String,
    ) -> broadcast::Sender<ChannelMessage> {
        let mut agents = self.agents.lock().await;
        if !agents.contains(&agent_id) {
            agents.push(agent_id);
            self.count.fetch_add(1, Ordering::SeqCst);
        }
        self.sender.clone()
    }

    pub async fn leave(&self, agent: String) {
        let mut agents = self.agents.lock().await;
        if let Some(pos) = agents.iter().position(|x| *x == agent) {
            agents.swap_remove(pos);
            self.count.fetch_sub(1, Ordering::SeqCst);
        }
    }

    /// broadcast messages to the channel
    /// it returns the number of agents who received the message
    pub fn send(
        &self,
        data: ChannelMessage,
    ) -> Result<usize, broadcast::error::SendError<ChannelMessage>> {
        self.sender.send(data)
    }

    pub fn empty(&self) -> bool {
        self.count.load(Ordering::SeqCst) == 0
    }

    pub async fn agents(&self) -> tokio::sync::MutexGuard<Vec<String>> {
        self.agents.lock().await
    }
}

impl ChannelControl {
    pub fn new() -> Self {
        ChannelControl {
            channel_map: Mutex::new(HashMap::new()),
            agent_task_map: Mutex::new(HashMap::new()),
            agent_sender_map: Mutex::new(HashMap::new()),
            conn_sender_map: Mutex::new(HashMap::new()),
        }
    }

    pub async fn add_connection(&self, name: String) {
        let mut conn_sender_map = self.conn_sender_map.lock().await;
        match conn_sender_map.entry(name.clone()) {
            Entry::Vacant(entry) => {
                let (tx, _rx) = broadcast::channel(100);
                entry.insert(tx);
                debug!("conn {} added", name.clone());
            }
            Entry::Occupied(_) => {}
        }
    }

    pub async fn remove_connection(&self, _name: String) {}
    pub async fn get_conn_subscription(
        &self,
        conn_id: String,
    ) -> Result<broadcast::Receiver<ChannelMessage>, ChannelError> {
        info!("get conn {} subscription", conn_id);
        let conn_sender_map = self.conn_sender_map.lock().await;
        let rx = conn_sender_map.get(&conn_id).unwrap().subscribe();
        Ok(rx)
    }

    pub async fn get_conn_sender(
        &self,
        conn_id: String,
    ) -> Result<broadcast::Sender<ChannelMessage>, ChannelError> {
        info!("get conn {} sender", conn_id);
        let conn_sender_map = self.conn_sender_map.lock().await;
        Ok(conn_sender_map.get(&conn_id).unwrap().clone())
    }

    pub async fn new_channel(&self, name: String, capacity: Option<usize>) {
        let mut channels = self.channel_map.lock().await;
        channels.insert(name.clone(), Channel::new(name, capacity));
    }

    pub async fn remove_channel(&self, channel_name: String) {
        match self.channel_map.lock().await.entry(channel_name.clone()) {
            Entry::Vacant(_) => {}
            Entry::Occupied(el) => {
                for agent in el.get().agents().await.iter() {
                    if let Entry::Occupied(mut agent_tasks) =
                        self.agent_task_map.lock().await.entry(agent.into())
                    {
                        let vecotr = agent_tasks.get_mut();
                        vecotr.retain(|task| {
                            if task.channel_name == channel_name {
                                task.join_task.abort();
                            }
                            task.channel_name != channel_name
                        });
                    }
                }

                el.remove();
            }
        }
    }

    pub async fn send_to_connction(
        &self,
        conn_id: String,
        message: ChannelMessage,
    ) -> Result<usize, ChannelError> {
        self.conn_sender_map
            .lock()
            .await
            .get(&conn_id)
            .ok_or(ChannelError::ChannelNotFound)?
            .send(message)
            .map_err(|_| ChannelError::MessageSendError)
    }

    /// broadcast message to the channel
    /// it returns the number of agents who received the message
    pub async fn broadcast(
        &self,
        channel_name: String,
        message: ChannelMessage,
    ) -> Result<usize, ChannelError> {
        self.channel_map
            .lock()
            .await
            .get(&channel_name)
            .ok_or(ChannelError::ChannelNotFound)?
            .send(message)
            .map_err(|_| ChannelError::MessageSendError)
    }

    // pub async fn get_agent_sender(
    //     &self,
    //     agent_id: String,
    // ) -> Result<broadcast::Sender<ChannelMessage>, ChannelError> {
    //     info!("get agent {} sender", agent_id);
    //     let agent_sender_map = self.agent_sender_map.lock().await;
    //     Ok(agent_sender_map.get(&agent_id).unwrap().clone())
    // }

    pub async fn get_agent_subscription(
        &self,
        agent_id: String,
    ) -> Result<broadcast::Receiver<ChannelMessage>, ChannelError> {
        info!("get agent {} receiver", agent_id);
        let agent_sender_map = self.agent_sender_map.lock().await;
        let receiver = agent_sender_map
            .get(&agent_id)
            .ok_or(ChannelError::AgentNotInitiated)?
            .subscribe();
        Ok(receiver)
    }

    /// Add channel agent to the channel ctl
    /// `capacity` is the maximum number of messages that can be stored in the channel, default is 100
    /// This will create a broadcast channel: ChannelAgent will write to and websocket_tx_task will
    /// subscribe to and read from
    pub async fn add_agent(&self, agent_id: String, capacity: Option<usize>) {
        let mut agent_sender_map = self.agent_sender_map.lock().await;
        match agent_sender_map.entry(agent_id.clone()) {
            Entry::Vacant(entry) => {
                let (tx, _rx) = broadcast::channel(capacity.unwrap_or(100));
                entry.insert(tx);
                info!("agent {} added", agent_id.clone());
            }
            Entry::Occupied(_) => {
                info!("agent {} already exists", agent_id.clone());
            }
        }
    }

    /// remove the agent after leaving all channels
    pub async fn remove_agent(&self, agent_id: String) {
        let channels = self.channel_map.lock().await;
        let mut agent_tasks = self.agent_task_map.lock().await;
        let mut agent_sender_map = self.agent_sender_map.lock().await;

        match agent_tasks.entry(agent_id.clone()) {
            Entry::Occupied(agent_tasks) => {
                let tasks = agent_tasks.get();
                for task in tasks {
                    let channel = channels.get(&task.channel_name);
                    if let Some(channel) = channel {
                        channel.leave(agent_id.clone()).await;
                        debug!(
                            "agent {} removed from channel {}",
                            agent_id, task
                        )
                    }
                    task.join_task.abort();
                }
                agent_tasks.remove();
                debug!("agent {} tasks removed", agent_id);
            }
            Entry::Vacant(_) => {}
        }

        match agent_sender_map.entry(agent_id.clone()) {
            Entry::Occupied(entry) => {
                entry.remove();
                debug!("agent {} receiver removed", agent_id);
            }
            Entry::Vacant(_) => {}
        }
    }

    /// join agent to channel
    /// This will subscribe to the channel, create a task to forward messages to the agent websocket
    pub async fn join_channel(
        &self,
        channel_name: &str,
        agent_id: String,
    ) -> Result<broadcast::Sender<ChannelMessage>, ChannelError> {
        let channel_map = self.channel_map.lock().await;
        let mut agent_task_map = self.agent_task_map.lock().await;
        let agent_sender_map = self.agent_sender_map.lock().await;

        let channel_sender = channel_map
            .get(channel_name)
            .ok_or(ChannelError::ChannelNotFound)?
            .join(agent_id.clone())
            .await;
        let channel_sub = channel_sender.subscribe();
        let agent_tx = agent_sender_map
            .get(&agent_id)
            .ok_or(ChannelError::AgentNotInitiated)?
            .clone();

        // a task for this join channel subscription to agent sender
        let join_task =
            tokio::spawn(channel_sub_to_agent(channel_sub, agent_tx));

        match agent_task_map.entry(agent_id.clone()) {
            Entry::Occupied(mut entry) => {
                let agent_tasks = entry.get_mut();
                if !agent_tasks.iter().any(|x| x.channel_name == channel_name) {
                    agent_tasks.push(ChannelAgent {
                        channel_name: channel_name.to_string().clone(),
                        join_task,
                    });
                }
            }
            Entry::Vacant(v) => {
                v.insert(vec![ChannelAgent {
                    channel_name: channel_name.to_string().clone(),
                    join_task,
                }]);
            }
        };
        Ok(channel_sender)
    }

    pub async fn leave_channel(
        &self,
        name: String,
        agent: String,
    ) -> Result<(), ChannelError> {
        let channels = self.channel_map.lock().await;
        let mut agents = self.agent_task_map.lock().await;

        channels
            .get(&name)
            .ok_or(ChannelError::ChannelNotFound)?
            .leave(agent.clone())
            .await;

        match agents.entry(agent.clone()) {
            Entry::Occupied(mut o) => {
                let vecotr = o.get_mut();
                vecotr.retain(|task| {
                    if task.channel_name == name {
                        task.join_task.abort();
                    }
                    task.channel_name != name
                });
            }
            Entry::Vacant(_) => {}
        }
        Ok(())
    }
}

impl Default for ChannelControl {
    fn default() -> Self {
        Self::new()
    }
}

fn build_eval_context(m: &ReplyMessage) -> HashMapContext {
    // context is built based on current TimedMessage
    let mut ctx = HashMapContext::new();
    if let Response::Jet1090 { timed_message } = &m.payload.response {
        // timesource
        let value: &str = match timed_message.timesource {
            TimeSource::System => "system",
            TimeSource::Radarcape => "radercape",
            TimeSource::External => "external",
        };
        let _ = ctx.set_value("timesource".into(), value.into());
        let _ =
            ctx.set_value("idx".into(), Value::Int(timed_message.idx as i64));

        let message = &<Option<rs1090::decode::Message> as Clone>::clone(
            &timed_message.message,
        )
        .unwrap();

        let mut icao24: String = "".to_string();
        let mut df: u16 = 0;
        let mut altitude: u16 = 0;
        match &message.df {
            DF::ShortAirAirSurveillance { ac, ap, .. } => {
                icao24 = format!("{}", ap);
                df = 0;
                altitude = ac.0;
            }
            DF::SurveillanceAltitudeReply { ac, ap, .. } => {
                icao24 = format!("{}", ap);
                df = 4;
                altitude = ac.0;
            }
            DF::SurveillanceIdentityReply { ap, .. } => {
                icao24 = format!("{}", ap);
                df = 5;
                altitude = 0;
            }
            DF::AllCallReply { icao, .. } => {
                icao24 = format!("{}", icao);
                df = 11;
                altitude = 0;
            }
            DF::LongAirAirSurveillance { ac, ap, .. } => {
                icao24 = format!("{}", ap);
                df = 16;
                altitude = ac.0;
            }
            DF::ExtendedSquitterADSB(adsb) => {
                icao24 = format!("{}", &adsb.icao24);
                df = 17;
                altitude = 0;
            }
            DF::ExtendedSquitterTisB { cf, .. } => {
                icao24 = format!("{}", cf.aa);
                df = 18;
                altitude = 0;
            }
            DF::ExtendedSquitterMilitary { .. } => {
                icao24 = "".to_string();
                df = 19;
                altitude = 0;
            }
            DF::CommBAltitudeReply { ac, ap, .. } => {
                icao24 = format!("{}", ap);
                df = 20;
                altitude = ac.0;
            }
            DF::CommBIdentityReply { ap, .. } => {
                icao24 = format!("{}", ap);
                df = 21;
                altitude = 0;
            }
            DF::CommDExtended { parity, .. } => {
                icao24 = format!("{}", parity);
                df = 24;
                altitude = 0;
            }
        };
        let _ = ctx.set_value("icao24".into(), icao24.into());
        let _ = ctx.set_value("df".into(), Value::Int(df.into()));
        let _ = ctx.set_value("altitude".into(), Value::Int(altitude.into()));
    }
    ctx
}

async fn channel_sub_to_agent(
    mut channel_sub_rx: broadcast::Receiver<ChannelMessage>,
    agent_tx: broadcast::Sender<ChannelMessage>,
) {
    // TODO: per agent per channel configurable
    // let code = r#"timesource == "system" && df == "0""#;
    let code = r#"
        true
        // (df == 0 || df == 4) && 
        // (altitude >= 30000)
        // (altitude >= 30000 && altitude <= 32000)
    "#;
    let mut operator_node = build_operator_tree(code).unwrap();

    while let Ok(channel_message) = channel_sub_rx.recv().await {
        match &channel_message {
            ChannelMessage::ReloadFilter { code, .. } => {
                info!("reloading filter ...");
                match build_operator_tree(code) {
                    Ok(node) => {
                        operator_node = node;
                        info!("filter reloaded");
                    }
                    Err(e) => {
                        error!("failed to reload filter: {}", e);
                    }
                }
            }
            ChannelMessage::Reply(reply_message) => {
                let ctx = build_eval_context(reply_message);
                match operator_node.eval_with_context(&ctx) {
                    Ok(Value::Boolean(true)) => {
                        let _ = agent_tx.send(channel_message);
                    }
                    Ok(_) => continue,
                    Err(e) => {
                        error!("failed to evaluate: {}", e);
                    }
                }
            }
        }
    }
    error!("failed to send to channel");
}

async fn websocket_sender(
    conn_id: String,
    state: Arc<ChannelState>,
    mut ws_tx: SplitSink<WebSocket, Message>,
) {
    debug!("launch websocket sender ...");
    let mut conn_rx = state
        .ctl
        .lock()
        .await
        .get_conn_subscription(conn_id.to_string())
        .await
        .unwrap();

    while let Ok(channel_message) = conn_rx.recv().await {
        if let ChannelMessage::Reply(reply_message) = channel_message {
            let text = serde_json::to_string(&reply_message).unwrap();
            error!("send {}", text);
            let result = ws_tx.send(warp::ws::Message::text(text)).await;
            if result.is_err() {
                error!("sending failure: {:?}", result);
                continue;
            }
        }
    }
}

async fn handle_events(
    mut ws_rx: SplitStream<WebSocket>,
    state: Arc<ChannelState>,
    conn_id: String,
) {
    info!("handle events ...");
    while let Some(Ok(m)) = ws_rx.next().await {
        if !m.is_text() {
            continue;
        }
        error!("conn_id {}", conn_id);
        // TODO do not crash
        let rm: RequestMessage =
            serde_json::from_str(m.to_str().unwrap()).unwrap();

        let reference = &rm.reference;
        let join_reference = &rm.join_reference;
        let channel = &rm.topic;
        let event = &rm.event;

        if channel == "phoenix" && event == "heartbeat" {
            debug!("heartbeat message");
            reply_ok_with_empty_response(
                conn_id.clone(),
                None,
                reference,
                "phoenix",
                state.clone(),
            )
            .await;
        }

        if event == "phx_join" {
            let _channel_foward_task =
                handle_join_event(&rm, state.clone(), &conn_id).await;
        }

        if event == "phx_leave" {
            state
                .ctl
                .lock()
                .await
                .leave_channel(channel.clone(), conn_id.to_string())
                .await
                .unwrap();
            reply_ok_with_empty_response(
                conn_id.clone(),
                join_reference.clone(),
                reference,
                channel,
                state.clone(),
            )
            .await;
        }
    }
}

async fn reply_ok_with_empty_response(
    conn_id: String,
    join_ref: Option<String>,
    event_ref: &str,
    channel_name: &str,
    state: Arc<ChannelState>,
) {
    let join_reply_message = ReplyMessage {
        join_reference: join_ref.clone(),
        reference: event_ref.to_string(),
        topic: channel_name.to_string(),
        event: "phx_reply".to_string(),
        payload: ReplyPayload {
            status: "ok".to_string(),
            response: Response::Empty {},
        },
    };
    let text = serde_json::to_string(&join_reply_message).unwrap();
    debug!(
        "sending empty response, channel: {}, join_ref: {:?}, ref: {}, {}",
        channel_name, join_ref, event_ref, text
    );
    state
        .ctl
        .lock()
        .await
        .send_to_connction(
            conn_id.to_string(),
            ChannelMessage::Reply(Box::new(join_reply_message)),
        )
        .await
        .unwrap();
    debug!("sent to connection {}: {}", conn_id.clone(), text);
}

async fn handle_join_event(
    rm: &RequestMessage,
    //ws_rx: &mut SplitStream<WebSocket>,
    state: Arc<ChannelState>,
    conn_id: &str,
) -> JoinHandle<()> {
    let channel_name = &rm.topic; // ?

    let agent_id =
        format!("{}:{}", conn_id, rm.join_reference.clone().unwrap());
    error!("{} joining {} ...", agent_id, channel_name.clone(),);
    state
        .ctl
        .lock()
        .await
        .add_agent(agent_id.to_string(), None)
        .await;
    state
        .ctl
        .lock()
        .await
        .join_channel(&channel_name.clone(), agent_id.to_string())
        .await
        .unwrap();
    // task to forward from agent broadcast to conn
    //let local_state = state.clone();
    let channel_forward_task = tokio::spawn(agent_rx_to_conn(
        state.clone(),
        rm.join_reference.clone().unwrap(),
        agent_id.clone(),
        conn_id.to_string(),
    ));
    reply_ok_with_empty_response(
        conn_id.to_string().clone(),
        rm.join_reference.clone(),
        &rm.reference,
        channel_name,
        state.clone(),
    )
    .await;
    channel_forward_task
}

async fn agent_rx_to_conn(
    state: Arc<ChannelState>,
    join_ref: String,
    agent_id: String,
    conn_id: String,
) {
    let mut agent_rx = state
        .ctl
        .lock()
        .await
        .get_agent_subscription(agent_id.clone())
        .await
        .unwrap();
    let conn_tx = state
        .ctl
        .lock()
        .await
        .get_conn_sender(conn_id.clone())
        .await
        .unwrap();
    error!(
        "forward agent {} to conn {} ...",
        agent_id.clone(),
        conn_id.clone()
    );
    while let Ok(mut channel_message) = agent_rx.recv().await {
        if let ChannelMessage::Reply(ref mut reply_message) = channel_message {
            reply_message.join_reference = Some(join_ref.clone());
            let result = conn_tx.send(channel_message.clone());
            if result.is_err() {
                error!("sending failure: {:?}", result.err().unwrap());
                break; // fails when there's no reciever, stop forwarding
            }
            info!("F {:?}", channel_message);
        }
    }
}

/*pub async fn system_datetime_task(state: Arc<State>, channel_name: &str) {
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    info!("launch system/datetime task...");
    let mut counter = 0;
    let event = "datetime";
    loop {
        let now = chrono::Local::now();
        let message = ReplyMessage {
            join_reference: None,
            reference: counter.to_string(),
            topic: channel_name.to_string(),
            event: event.to_string(),
            payload: ReplyPayload {
                status: "ok".to_string(),
                response: Response::Datetime {
                    datetime: now
                        .to_rfc3339_opts(chrono::SecondsFormat::Millis, false),
                    counter,
                },
            },
        };
        let text = serde_json::to_string(&message).unwrap();
        match state
            .ctl
            .lock()
            .await
            .broadcast(
                channel_name.to_string(),
                ChannelMessage::Reply(Box::new(message)),
            )
            .await
        {
            Ok(0) => {} // no client
            Ok(_) => {} // debug!("datetime > {}", text),
            Err(e) => {
                // FIXME: when thers's no client, it's an error
                // error!("`{}` `{}`, {}, {}", channel_name, event, e, text)
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        counter += 1;
    }
}
*/
