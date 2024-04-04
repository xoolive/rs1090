use crate::snapshot::Snapshot;
use crate::Jet1090;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::http::StatusCode;
use warp::reject::Rejection;
use warp::reply::Reply;

#[derive(Serialize, Deserialize)]
pub struct TrackQuery {
    icao24: String,
}

/// An API error serializable to JSON.
#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}

pub async fn icao24(
    app: &Arc<Mutex<Jet1090>>,
) -> Result<warp::reply::Json, Infallible> {
    let app = app.lock().await;
    Ok::<_, Infallible>(warp::reply::json(&app.items))
}

pub async fn all(
    app: &Arc<Mutex<Jet1090>>,
) -> Result<warp::reply::Json, Infallible> {
    let app = app.lock().await;
    Ok::<_, Infallible>(warp::reply::json(
        &app.state_vectors
            .values()
            .map(|sv| &sv.cur)
            .collect::<Vec<&Snapshot>>(),
    ))
}

pub async fn track(
    app: &Arc<Mutex<Jet1090>>,
    q: TrackQuery,
) -> Result<warp::reply::Json, Infallible> {
    let app = app.lock().await;
    Ok::<_, Infallible>(warp::reply::json(
        &app.state_vectors.get(&q.icao24).map(|sv| &sv.hist),
    ))
}

// Define a rejection handler
pub async fn handle_rejection(
    err: Rejection,
) -> Result<impl Reply, Infallible> {
    // https://github.com/seanmonstar/warp/blob/master/examples/rejections.rs

    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message =
            "Route not found, try one of / /all and /track?icao24={icao24}";
    } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
        code = StatusCode::METHOD_NOT_ALLOWED;
        message = "Only GET queries are supported";
    } else if err.find::<warp::reject::InvalidQuery>().is_some() {
        code = StatusCode::BAD_REQUEST;
        message = "Invalid query";
    } else {
        eprintln!("unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "Unknown error";
    }

    let json = warp::reply::json(&ErrorMessage {
        code: code.as_u16(),
        message: message.into(),
    });

    Ok(warp::reply::with_status(json, code))
}
