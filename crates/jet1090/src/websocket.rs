use std::path::PathBuf;
use std::sync::Arc;
use std::{
    fmt::{Display, Error},
    net::SocketAddr,
};

use futures::stream::StreamExt;
use log::{debug, error, info, warn};
use rs1090::prelude::TimedMessage;
use serde::{Deserialize, Serialize};
use serde_tuple::{Deserialize_tuple, Serialize_tuple};
use tokio::sync::{
    mpsc::{Receiver, UnboundedReceiver},
    Mutex,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tower_http::services::ServeDir;
use uuid::Uuid;

use crate::channels::{ChannelControl, ChannelMessage};

/// reply data structures
#[derive(Clone, Debug, Serialize_tuple)]
pub struct ReplyMessage {
    join_reference: Option<String>, // null when it's heartbeat
    reference: String,
    topic: String, // `channel`
    event: String,
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
    Join {},
    Heartbeat {},
    Datetime { datetime: String, counter: u32 },
    Jet1090 { timed_message: TimedMessage },
}

/// request data structures

/// RequestMessage is a message from client through websocket
/// it's deserialized from a JSON array
#[derive(Debug, Deserialize_tuple)]
struct RequestMessage {
    join_reference: Option<String>, // null when it's heartbeat
    reference: String,
    topic: String, // `channel`
    event: String,
    payload: RequestPayload,
}

impl Display for RequestMessage {
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> Result<(), Error> {
        write!(
            formatter,
            "<RequestMessage: join_ref={:?}, ref={}, topic={}, event={}, payload=...>",
            self.join_reference, self.reference, self.topic, self.event
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RequestPayload {
    Join { token: String },
    Leave {},
    Heartbeat {},
}

pub struct User {
    pub user_id: String,
    pub session_id: i32,
}

impl Default for User {
    fn default() -> Self {
        User {
            user_id: "0".to_string(), // Uuid::new_v4().to_string(),
            session_id: 0,
        }
    }
}

// enum MyMessage {
//     String,
// }

pub struct State {
    pub channels: Mutex<ChannelControl>,
}

pub async fn rs1090_data_task(
    local_state: Arc<State>,
    mut data_source: UnboundedReceiverStream<TimedMessage>,
    channel_name: &str,
    event_name: &str,
) {
    let mut counter = 0;
    while let Some(timed_message) = data_source.next().await {
        let reply_message = ReplyMessage {
            join_reference: None,
            reference: counter.to_string(),
            topic: channel_name.to_string(),
            event: event_name.to_string(),
            payload: ReplyPayload {
                status: "ok".to_string(),
                response: Response::Jet1090 { timed_message },
            },
        };
        let text = serde_json::to_string(&reply_message).unwrap();
        match local_state
            .channels
            .lock()
            .await
            .broadcast(
                channel_name.to_string(),
                ChannelMessage::Reply(reply_message),
            )
            .await
        {
            Ok(_) => {
                counter += 1;
                debug!("{} > {}", event_name, text);
            }
            Err(e) => {
                // it throws error if there's no client
                // error!(
                //     "fail to send, channel: {}, event: {}, err: {}",
                //     channel_name, event_name, e
                // );
            }
        }
    }
}

pub async fn timestamp_task(local_state: Arc<State>, channel_name: &str) {
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    info!("launch datetime thread ...");
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
        match local_state
            .channels
            .lock()
            .await
            .broadcast(channel_name.to_string(), ChannelMessage::Reply(message))
            .await
        {
            Ok(0) => {} // no client
            Ok(_) => debug!("datetime > {}", text),
            Err(e) => {
                warn!("fail to send, err: {}", e)
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        counter += 1;
    }
}

async fn reply_ok_with_empty_response(
    join_reference: Option<String>,
    reference: &str,
    channel: &str,
    state: Arc<State>,
) {
    let join_reply_message = ReplyMessage {
        join_reference: join_reference.clone(),
        reference: reference.to_string(),
        topic: channel.to_string(),
        event: "phx_reply".to_string(),
        payload: ReplyPayload {
            status: "ok".to_string(),
            response: Response::Empty {},
        },
    };
    let text = serde_json::to_string(&join_reply_message).unwrap();
    state
        .channels
        .lock()
        .await
        .broadcast(
            channel.to_string(),
            ChannelMessage::Reply(join_reply_message),
        )
        .await
        .unwrap();
    debug!("> {}", text);
}

pub async fn handle_incoming_messages(
    received_text: String,
    state: Arc<State>,
    user_id: &str,
) {
    debug!("< {}", received_text);

    let received_message: RequestMessage =
        serde_json::from_str(&received_text).unwrap();
    debug!("{}", received_message);

    let reference = &received_message.reference;
    let join_reference = &received_message.join_reference;
    let channel = &received_message.topic;
    let event = &received_message.event;

    if event == "phx_join" {
        state
            .channels
            .lock()
            .await
            .add_user(user_id.to_string(), None)
            .await;
        state
            .channels
            .lock()
            .await
            .join_channel("phoenix".into(), user_id.into())
            .await
            .unwrap();
        state
            .channels
            .lock()
            .await
            .join_channel(channel.clone(), user_id.to_string())
            .await
            .unwrap(); // join user to system channel
        reply_ok_with_empty_response(
            join_reference.clone(),
            reference,
            channel,
            state.clone(),
        )
        .await;
    }

    if event == "phx_leave" {
        state
            .channels
            .lock()
            .await
            .leave_channel(channel.clone(), user_id.to_string())
            .await
            .unwrap();
        reply_ok_with_empty_response(
            join_reference.clone(),
            reference,
            channel,
            state.clone(),
        )
        .await;
    }

    if channel == "phoenix" && event == "heartbeat" {
        debug!("heartbeat message");
        reply_ok_with_empty_response(
            Option::None,
            reference,
            "phoenix",
            state.clone(),
        )
        .await;
    }
}
