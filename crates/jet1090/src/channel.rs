use std::{
    collections::{hash_map::Entry, HashMap},
    error::Error,
    fmt::{self, Display},
    sync::atomic::{AtomicU32, Ordering},
};

use serde::Serialize;
use tokio::{
    sync::{broadcast, Mutex},
    task::JoinHandle,
};
use tracing::{debug, error, info};

use evalexpr::{
    build_operator_tree, context_map, Context, ContextWithMutableVariables,
    HashMapContext, Value,
};

use crate::websocket::{ReplyMessage, Response};
use rs1090::decode::DF;
use rs1090::decode::{TimeSource, TimedMessage};

#[derive(Clone, Debug, Serialize)]
pub enum ChannelMessage {
    Reply(ReplyMessage),
    ReloadFilter { agent_id: String, code: String },
}

/// agent channel, can broadcast to every agent in the channel
pub struct Channel {
    /// channel name
    pub name: String,
    /// broadcast in channels
    sender: broadcast::Sender<ChannelMessage>,
    /// channel agents
    agents: Mutex<Vec<String>>,
    /// channel agent count
    count: AtomicU32,
}

/// manages all channels
pub struct ChannelControl {
    pub channel_map: Mutex<HashMap<String, Channel>>, // channel name -> Channel

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

impl Error for ChannelError {}

impl fmt::Display for ChannelError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ChannelError::ChannelNotFound => {
                write!(f, "<ChannelNotFound: channel not found>")
            }
            ChannelError::AgentNotInitiated => {
                write!(f, "<AgentNotInitiated: agent not initiated>")
            }
            ChannelError::MessageSendError => {
                write!(
                    f,
                    "<MessageSendError: failed to send a message to the channel>"
                )
            }
        }
    }
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

    pub async fn remove_connection(&self, name: String) {}
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
        info!("get agent {} reciever", agent_id);
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
        let mut channel_sub = channel_sender.subscribe();
        let agent_tx = agent_sender_map
            .get(&agent_id)
            .ok_or(ChannelError::AgentNotInitiated)?
            .clone();

        /// a task for this join
        /// channel subscription to agent sender
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
        ctx.set_value("timesource".into(), value.into());
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
        ctx.set_value("icao24".into(), icao24.into());
        ctx.set_value("df".into(), Value::Int(df.into()));
        ctx.set_value("altitude".into(), Value::Int(altitude.into()));
    }
    ctx
}

async fn channel_sub_to_agent(
    mut channel_sub_rx: broadcast::Receiver<ChannelMessage>,
    agent_tx: broadcast::Sender<ChannelMessage>,
) {
    // TODO: per agent per channel configurable
    // let code = r#"timesource == "system" && df == "0""#;
    let mut code = r#"
        true
        // (df == 0 || df == 4) && 
        // (altitude >= 30000)
        // (altitude >= 30000 && altitude <= 32000)
    "#;
    let mut operator_node = build_operator_tree(code).unwrap();

    while let Ok(channel_message) = channel_sub_rx.recv().await {
        match &channel_message {
            ChannelMessage::ReloadFilter { agent_id, code } => {
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
                        agent_tx.send(channel_message);
                    }
                    Ok(_) => continue,
                    Err(e) => {
                        error!("failed to evaluate: {}", e);
                    }
                }
            }
        }
    }
}
