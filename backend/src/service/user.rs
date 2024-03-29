use crate::database::user;
use crate::error::ErrorCode;
use crate::server::Server;
use crate::{Future, Result};
use futures::future::{Future as _, IntoFuture};
use hyper::{header::HeaderValue, HeaderMap};
use slog::{info, Logger};
use std::sync::Arc;
use uuid::Uuid;

fn get_session(headers: &HeaderMap<HeaderValue>) -> Result<String> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|a| a.to_owned())
        .ok_or_else(|| ErrorCode::NotAuthenticated.default())
}

pub fn add_user(log: Logger, server: Server, username: String, password: String) -> Future<()> {
    info!(log, "Adding user | username: {}", username);
    Box::new(
        server
            .database
            .run(move |pool| user::add_user(pool, &username, &password)),
    )
}

pub fn create_session(server: Server, username: String, password: String) -> Future<String> {
    let uuid = Arc::new(Uuid::new_v4().to_string());
    let uuid1 = uuid.clone();
    Box::new(
        server
            .database
            .run(move |pool| user::match_password(pool, &username, &password))
            .and_then(move |user_id| server.sled.session.save(&uuid, user_id))
            .map(move |_| uuid1.to_string()),
    )
}

pub fn from_session_id(server: &Server, session_id: String) -> Result<i64> {
    Ok(session_id).and_then(move |sid| match server.sled.session.get(&sid) {
        Ok(Some(user_id)) => Ok(user_id),
        Ok(None) => ErrorCode::NotAuthenticated.default().err(),
        Err(err) => Err(err),
    })
}

pub fn from_header(server: &Server, headers: &HeaderMap<HeaderValue>) -> Result<i64> {
    get_session(headers).and_then(|sid| from_session_id(server, sid))
}

pub fn remove_session(server: Server, session_id: String) -> Future<()> {
    Box::new(
        Ok(session_id)
            .into_future()
            .and_then(move |sid| server.sled.session.del(&sid)),
    )
}
