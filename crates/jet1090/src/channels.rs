use std::{
    collections::{hash_map::Entry, HashMap},
    error::Error,
    fmt::{self, Display},
    sync::atomic::{AtomicU32, Ordering},
};

use log::{debug, error, info};
use serde::Serialize;
use tokio::{
    sync::{broadcast, Mutex},
    task::JoinHandle,
};

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
    ReloadFilter(String),
}

/// user channel, can broadcast to every user in the channel
pub struct Channel {
    /// channel name
    pub name: String,
    /// broadcast in channels
    sender: broadcast::Sender<ChannelMessage>,
    /// channel users
    users: Mutex<Vec<String>>,
    /// channel user count
    count: AtomicU32,
}

/// manages all channels
pub struct ChannelControl {
    pub channel_map: Mutex<HashMap<String, Channel>>, // channel name -> Channel
    user_task_map: Mutex<HashMap<String, Vec<UserTask>>>, // user_id -> threads
    user_sender_map: Mutex<HashMap<String, broadcast::Sender<ChannelMessage>>>, // user_id -> Sender
}

#[derive(Debug)]
pub enum ChannelError {
    /// channel does not exist
    ChannelNotFound,
    /// can not send message to channel
    MessageSendError,
    /// you have not called init_user
    UserNotInitiated,
}

impl Error for ChannelError {}

impl fmt::Display for ChannelError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ChannelError::ChannelNotFound => {
                write!(f, "channel not found")
            }
            ChannelError::UserNotInitiated => {
                write!(f, "user not initiated")
            }
            ChannelError::MessageSendError => {
                write!(f, "failed to send a message to the channel")
            }
        }
    }
}

struct UserTask {
    channel_name: String,
    task: JoinHandle<()>,
}

impl Display for UserTask {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "<UserTask: channel_name={}, task={:?}>",
            self.channel_name, self.task
        )
    }
}

impl Channel {
    pub fn new(name: String, capacity: Option<usize>) -> Channel {
        let (tx, _rx) = broadcast::channel(capacity.unwrap_or(100));
        Channel {
            name,
            sender: tx,
            users: Mutex::new(vec![]),
            count: AtomicU32::new(0),
        }
    }

    pub async fn join(
        &self,
        user: String,
    ) -> broadcast::Sender<ChannelMessage> {
        let mut users = self.users.lock().await;
        if !users.contains(&user) {
            users.push(user);
            self.count.fetch_add(1, Ordering::SeqCst);
        }
        self.sender.clone()
    }

    pub async fn leave(&self, user: String) {
        let mut users = self.users.lock().await;
        if let Some(pos) = users.iter().position(|x| *x == user) {
            users.swap_remove(pos);
            self.count.fetch_sub(1, Ordering::SeqCst);
        }
    }

    /// broadcast messages to the channel
    /// it returns the number of users who received the message
    pub fn send(
        &self,
        data: ChannelMessage,
    ) -> Result<usize, broadcast::error::SendError<ChannelMessage>> {
        self.sender.send(data)
    }

    pub fn empty(&self) -> bool {
        self.count.load(Ordering::SeqCst) == 0
    }

    pub async fn users(&self) -> tokio::sync::MutexGuard<Vec<String>> {
        self.users.lock().await
    }
}

impl ChannelControl {
    pub fn new() -> Self {
        ChannelControl {
            channel_map: Mutex::new(HashMap::new()),
            user_task_map: Mutex::new(HashMap::new()),
            user_sender_map: Mutex::new(HashMap::new()),
        }
    }

    pub async fn new_channel(&self, name: String, capacity: Option<usize>) {
        let mut channels = self.channel_map.lock().await;
        channels.insert(name.clone(), Channel::new(name, capacity));
    }

    pub async fn channel_exists(&self, name: &str) -> bool {
        let channels = self.channel_map.lock().await;
        channels.get(name).is_some()
    }

    pub async fn remove_channel(&self, channel: String) {
        let mut channels = self.channel_map.lock().await;
        let mut users = self.user_task_map.lock().await;
        match channels.entry(channel.clone()) {
            Entry::Vacant(_) => {}
            Entry::Occupied(el) => {
                for user in el.get().users().await.iter() {
                    if let Entry::Occupied(mut user_task) =
                        users.entry(user.into())
                    {
                        let vecotr = user_task.get_mut();
                        vecotr.retain(|task| {
                            if task.channel_name == channel {
                                task.task.abort();
                            }
                            task.channel_name != channel
                        });
                    }
                }

                el.remove();
            }
        }
    }

    /// broadcast message to the channel
    /// it returns the number of users who received the message
    pub async fn broadcast(
        &self,
        channel_name: String,
        message: ChannelMessage,
    ) -> Result<usize, ChannelError> {
        self.channel_map
            .lock()
            .await
            .get(&channel_name)
            .ok_or(ChannelError::ChannelNotFound)? // channel not found error
            .send(message)
            .map_err(|_| ChannelError::MessageSendError) // message send fail error
    }

    pub async fn get_user_sender(
        &self,
        user: String,
    ) -> Result<broadcast::Sender<ChannelMessage>, ChannelError> {
        info!("get user {} sender", user);
        let user_senders = self.user_sender_map.lock().await;
        Ok(user_senders.get(&user).unwrap().clone())
    }

    pub async fn get_user_receiver(
        &self,
        user: String,
    ) -> Result<broadcast::Receiver<ChannelMessage>, ChannelError> {
        let user_senders = self.user_sender_map.lock().await;
        let receiver = user_senders
            .get(&user)
            .ok_or(ChannelError::UserNotInitiated)?
            .subscribe();
        Ok(receiver)
    }

    /// Add user to the channel manager
    /// capacity is the maximum number of messages that can be stored in the channel, default is 100
    pub async fn add_user(&self, user: String, capacity: Option<usize>) {
        let mut user_senders = self.user_sender_map.lock().await;
        match user_senders.entry(user) {
            Entry::Vacant(entry) => {
                let (tx, _rx) = broadcast::channel(capacity.unwrap_or(100));
                entry.insert(tx);
            }
            Entry::Occupied(_) => {}
        }
    }

    /// call this at end of your code to remove user from all channels
    pub async fn remove_user(&self, user: String) {
        let channels = self.channel_map.lock().await;
        let mut user_tasks = self.user_task_map.lock().await;
        let mut user_receiver = self.user_sender_map.lock().await;

        match user_tasks.entry(user.clone()) {
            Entry::Occupied(user_tasks) => {
                let tasks = user_tasks.get();
                for task in tasks {
                    let channel = channels.get(&task.channel_name);
                    if let Some(channel) = channel {
                        channel.leave(user.clone()).await;
                        debug!("user {} removed from channel {}", user, task)
                    }
                    task.task.abort();
                }
                user_tasks.remove();
                debug!("user {} tasks removed", user);
            }
            Entry::Vacant(_) => {}
        }

        match user_receiver.entry(user.clone()) {
            Entry::Occupied(entry) => {
                entry.remove();
                debug!("user {} receiver removed", user);
            }
            Entry::Vacant(_) => {}
        }
    }

    /// join user to channel
    pub async fn join_channel(
        &self,
        channel_name: String,
        user: String,
    ) -> Result<broadcast::Sender<ChannelMessage>, ChannelError> {
        let channel_map = self.channel_map.lock().await;
        let mut user_task_map = self.user_task_map.lock().await;
        let user_sender_map = self.user_sender_map.lock().await;

        let channel_sender = channel_map
            .get(&channel_name)
            .ok_or(ChannelError::ChannelNotFound)?
            .join(user.clone())
            .await;
        let mut channel_subscription_receiver = channel_sender.subscribe();
        let user_sender = user_sender_map
            .get(&user)
            .ok_or(ChannelError::UserNotInitiated)?
            .clone();

        let task = tokio::spawn(user_receiver_to_websocket_sender(
            channel_subscription_receiver,
            user_sender,
        ));

        match user_task_map.entry(user.clone()) {
            Entry::Occupied(mut entry) => {
                let user_tasks = entry.get_mut();
                if !user_tasks.iter().any(|x| x.channel_name == channel_name) {
                    user_tasks.push(UserTask { channel_name, task });
                }
            }
            Entry::Vacant(v) => {
                v.insert(vec![UserTask { channel_name, task }]);
            }
        };
        Ok(channel_sender)
    }

    pub async fn leave_channel(
        &self,
        name: String,
        user: String,
    ) -> Result<(), ChannelError> {
        let channels = self.channel_map.lock().await;
        let mut users = self.user_task_map.lock().await;

        channels
            .get(&name)
            .ok_or(ChannelError::ChannelNotFound)?
            .leave(user.clone())
            .await;

        match users.entry(user.clone()) {
            Entry::Occupied(mut o) => {
                let vecotr = o.get_mut();
                vecotr.retain(|task| {
                    if task.channel_name == name {
                        task.task.abort();
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

        let message = &<Option<rs1090::decode::Message> as Clone>::clone(
            &timed_message.message,
        )
        .unwrap();

        // df
        let mut df: u16 = 0;
        let mut altitude: u16 = 0;
        match message.df {
            DF::ShortAirAirSurveillance { ac, .. } => {
                df = 0;
                altitude = ac.0;
            }
            DF::SurveillanceAltitudeReply { ac, .. } => {
                df = 4;
                altitude = ac.0;
            }
            DF::SurveillanceIdentityReply { .. } => {
                df = 5;
                altitude = 0;
            }
            DF::AllCallReply { .. } => {
                df = 11;
                altitude = 0;
            }
            DF::LongAirAirSurveillance { ac, .. } => {
                df = 16;
                altitude = ac.0;
            }
            DF::ExtendedSquitterADSB { .. } => {
                df = 17;
                altitude = 0;
            }
            DF::ExtendedSquitterTisB { .. } => {
                df = 18;
                altitude = 0;
            }
            DF::ExtendedSquitterMilitary { .. } => {
                df = 19;
                altitude = 0;
            }
            DF::CommBAltitudeReply { ac, .. } => {
                df = 20;
                altitude = ac.0;
            }
            DF::CommBIdentityReply { .. } => {
                df = 21;
                altitude = 0;
            }
            DF::CommDExtended { .. } => {
                df = 24;
                altitude = 0;
            }
        };
        ctx.set_value("df".into(), Value::Int(df.into()));
        ctx.set_value("altitude".into(), Value::Int(altitude.into()));
    }
    ctx
}

async fn user_receiver_to_websocket_sender(
    mut channel_subscription_receiver: broadcast::Receiver<ChannelMessage>,
    user_sender: broadcast::Sender<ChannelMessage>,
) {
    // TODO: per user per channel configurable
    // let code = r#"timesource == "system" && df == "0""#;
    let mut code = r#"
        true
        // (df == 0 || df == 4) && 
        // (altitude >= 30000)
        // (altitude >= 30000 && altitude <= 32000)
    "#;
    let mut operator_node = build_operator_tree(code).unwrap();

    while let Ok(channel_message) = channel_subscription_receiver.recv().await {
        match &channel_message {
            ChannelMessage::ReloadFilter(code) => {
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
                let ctx = build_eval_context(&reply_message);
                match operator_node.eval_with_context(&ctx) {
                    Ok(Value::Boolean(true)) => {
                        user_sender.send(channel_message);
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
