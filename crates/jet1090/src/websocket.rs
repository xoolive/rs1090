use futures::stream::SplitSink;
use futures::SinkExt;
use futures::StreamExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::{
    fmt::{Display, Error},
    net::SocketAddr,
};
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use warp::filters::ws::Message;

use futures::stream::SplitStream;
use rs1090::prelude::TimedMessage;
use serde::{Deserialize, Serialize};
use serde_tuple::{Deserialize_tuple, Serialize_tuple};
use std::collections::HashMap;
use tokio::select;
use tokio::sync::{
    broadcast::Receiver, mpsc::UnboundedReceiver, watch::Sender, Mutex,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tower_http::services::ServeDir;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use warp::filters::ws::WebSocket;

use crate::channel::{ChannelControl, ChannelError, ChannelMessage};

/// reply data structures
#[derive(Clone, Debug, Serialize_tuple)]
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

pub struct State {
    pub ctl: Mutex<ChannelControl>,
}

pub async fn on_connected(ws: WebSocket, state: Arc<State>) {
    let conn_id = Uuid::new_v4().to_string();
    info!("on_connected: {}", conn_id);

    state.ctl.lock().await.add_connection(conn_id.clone()).await;

    let (ws_tx, mut ws_rx) = ws.split();

    // spawn a taks to forward conn mpsc to websocket
    let mut ws_tx_task =
        tokio::spawn(websocket_sender(conn_id.clone(), state.clone(), ws_tx));

    // TODO: a task is created when handling joining event
    let mut ws_rx_task =
        tokio::spawn(handle_events(ws_rx, state.clone(), conn_id.clone()));

    tokio::select! {
        _ = (&mut ws_tx_task) => ws_rx_task.abort(),
        _ = (&mut ws_rx_task) => ws_tx_task.abort(),
    }

    state
        .ctl
        .lock()
        .await
        .remove_agent(conn_id.to_string())
        .await;
    info!("client connection closed");
}

async fn handle_join_event(
    rm: &RequestMessage,
    ws_rx: &mut SplitStream<WebSocket>,
    state: Arc<State>,
    conn_id: &str,
) -> JoinHandle<()> {
    let channel_name = &rm.topic; // ?

    let agent_id =
        format!("{}:{}", conn_id, rm.join_reference.clone().unwrap());
    info!("{} joining {} ...", agent_id, channel_name.clone(),);
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
    let local_state = state.clone();
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
    state: Arc<State>,
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
    debug!(
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
            debug!("F {:?}", channel_message);
        }
    }
}

async fn handle_events(
    mut ws_rx: SplitStream<WebSocket>,
    state: Arc<State>,
    conn_id: String,
) {
    info!("handle events ...");
    while let Some(Ok(m)) = ws_rx.next().await {
        if !m.is_text() {
            continue;
        }
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
                handle_join_event(&rm, &mut ws_rx, state.clone(), &conn_id)
                    .await;
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

async fn websocket_sender(
    conn_id: String,
    state: Arc<State>,
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
        if let ChannelMessage::Reply(mut reply_message) = channel_message {
            let text = serde_json::to_string(&reply_message).unwrap();
            let result = ws_tx.send(warp::ws::Message::text(text)).await;
            if result.is_err() {
                error!("sending failure: {:?}", result);
                continue;
            }
        }
    }
}

async fn reply_ok_with_empty_response(
    conn_id: String,
    join_ref: Option<String>,
    event_ref: &str,
    channel_name: &str,
    state: Arc<State>,
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
            ChannelMessage::Reply(join_reply_message),
        )
        .await
        .unwrap();
    debug!("sent to connection {}: {}", conn_id.clone(), text);
}

pub async fn jet1090_data_task(
    local_state: Arc<State>,
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
                response: Response::Jet1090 { timed_message },
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

pub async fn system_datetime_task(state: Arc<State>, channel_name: &str) {
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
            .broadcast(channel_name.to_string(), ChannelMessage::Reply(message))
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
