use std::{
    collections::{hash_map::Entry, HashMap},
    error::Error,
    fmt::{self, Display},
    sync::atomic::{AtomicU32, Ordering},
};

use log::debug;
use tokio::{
    sync::{broadcast, Mutex},
    task::JoinHandle,
};

use evalexpr::{
    build_operator_tree, context_map, Context, ContextWithMutableVariables,
    HashMapContext,
};

use crate::websocket::{ReplyMessage, Response};
use rs1090::decode::TimeSource;
use rs1090::decode::DF;

/// user channel, can broadcast to every user in the channel
pub struct Channel {
    /// channel name
    pub name: String,
    /// broadcast in channels
    sender: broadcast::Sender<ReplyMessage>,
    /// channel users
    users: Mutex<Vec<String>>,
    /// channel user count
    count: AtomicU32,
}

/// manages all channels
pub struct ChannelControl {
    channel_map: Mutex<HashMap<String, Channel>>, // name -> channel
    user_task_map: Mutex<HashMap<String, Vec<UserTask>>>,
    user_sender_map: Mutex<HashMap<String, broadcast::Sender<ReplyMessage>>>,
}

#[derive(Debug)]
pub enum ChannelError {
    /// channel does not exist
    ChannelNotFound,
    /// can not send message to channel
    MessageSendFail,
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
            ChannelError::MessageSendFail => {
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

    pub async fn join(&self, user: String) -> broadcast::Sender<ReplyMessage> {
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
        data: ReplyMessage,
    ) -> Result<usize, broadcast::error::SendError<ReplyMessage>> {
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
        message: ReplyMessage,
    ) -> Result<usize, ChannelError> {
        self.channel_map
            .lock()
            .await
            .get(&channel_name)
            .ok_or(ChannelError::ChannelNotFound)? // channel not found error
            .send(message)
            .map_err(|_| ChannelError::MessageSendFail) // message send fail error
    }

    pub async fn get_user_receiver(
        &self,
        user: String,
    ) -> Result<broadcast::Receiver<ReplyMessage>, ChannelError> {
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
    ) -> Result<broadcast::Sender<ReplyMessage>, ChannelError> {
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

        let task = tokio::spawn(async move {
            // data: channel => user
            let pred = |m: &ReplyMessage| -> bool {
                // TODO: per user per channel configurable
                let code = r#"timesource == "system" && df == "0""#;
                let precompiled_bytecodes = build_operator_tree(code).unwrap();

                let mut ctx = HashMapContext::new();
                if let Response::Jet1090 { timed_message } = &m.payload.response
                {
                    let value: &str = match timed_message.timesource {
                        TimeSource::System => "system",
                        TimeSource::Radarcape => "radercape",
                        TimeSource::External => "external",
                    };
                    ctx.set_value("timesource".into(), value.into());

                    let message =
                        &<Option<rs1090::decode::Message> as Clone>::clone(
                            &timed_message.message,
                        )
                        .unwrap();
                    let df = match message.df {
                        DF::ShortAirAirSurveillance { .. } => "0",
                        _ => "",
                    };
                    ctx.set_value("df".into(), df.into());
                }

                match precompiled_bytecodes.eval_with_context(&ctx) {
                    Ok(result) => result.as_boolean().unwrap(),
                    Err(e) => {
                        debug!("fail to evaluate: {}", e);
                        true
                    }
                }
            };
            while let Ok(data) = channel_subscription_receiver.recv().await {
                if pred(&data) {
                    let _ = user_sender.send(data);
                }
            }
        });

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
