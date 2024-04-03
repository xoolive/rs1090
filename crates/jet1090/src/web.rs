use crate::snapshot::Snapshot;
use crate::Jet1090;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::reject::Rejection;
use warp::reply::Reply;

#[derive(Serialize, Deserialize)]
pub struct TrackQuery {
    icao24: String,
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
    _err: Rejection,
) -> Result<impl Reply, Infallible> {
    Ok(warp::reply::with_status(
        warp::reply::json(&"Not found"),
        warp::http::StatusCode::NOT_FOUND,
    ))
}
