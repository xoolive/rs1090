use rs1090::data::airports::{Airport, AIRPORTS};
/**
 * Information returned on a REST API
 */
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::http::StatusCode;
use warp::reject::Rejection;
use warp::reply::Reply;

use crate::snapshot::Snapshot;
use crate::Jet1090;

/// Information required to ask for a trajectory
#[derive(Serialize, Deserialize)]
pub struct TrackQuery {
    icao24: String,
}

/// Information required to search for airports
#[derive(Serialize, Deserialize)]
pub struct Query {
    q: String,
}

/// An API error serializable to JSON
#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}

/// Returns all the ICAO 24-bit addresses of aircraft seen by jet1090
pub async fn icao24(
    app: &Arc<Mutex<Jet1090>>,
) -> Result<warp::reply::Json, Infallible> {
    let app = app.lock().await;
    Ok::<_, Infallible>(warp::reply::json(&app.items))
}

/// Returns all state vectors without any history information
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

/// Returns the trajectory of a given aircraft matching the REST query
pub async fn track(
    app: &Arc<Mutex<Jet1090>>,
    q: TrackQuery,
) -> Result<warp::reply::Json, Infallible> {
    let app = app.lock().await;
    Ok::<_, Infallible>(warp::reply::json(
        &app.state_vectors.get(&q.icao24).map(|sv| &sv.hist),
    ))
}

/// Returns decoding information about all sensors
pub async fn sensors(
    app: &Arc<Mutex<Jet1090>>,
) -> Result<warp::reply::Json, Infallible> {
    let app = app.lock().await;
    Ok::<_, Infallible>(warp::reply::json(&app.sensors))
}

/// Returns a list of poential airports matching the query string
pub async fn airports(query: Query) -> Result<warp::reply::Json, Infallible> {
    let lowercase = query.q.to_lowercase();
    let res: Vec<&Airport> = AIRPORTS
        .iter()
        .filter(|a| {
            a.name.to_lowercase().contains(&lowercase)
                || a.city.to_lowercase().contains(&lowercase)
                || a.icao.to_lowercase().contains(&lowercase)
                || a.iata.to_lowercase().contains(&lowercase)
        })
        .collect();
    Ok::<_, Infallible>(warp::reply::json(&res))
}

/// Returns proper error messages in JSON format
pub async fn handle_rejection(
    err: Rejection,
) -> Result<impl Reply, Infallible> {
    // https://github.com/seanmonstar/warp/blob/master/examples/rejections.rs

    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "Route not found, try one of /,\n\
            /all,\n\
            /track?icao24={icao24},\n\
            /sensors or\n\
            /airport?q={string}";
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
