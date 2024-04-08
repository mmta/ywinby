use std::{
    sync::{
        atomic::{
            AtomicBool,
            Ordering::{Acquire, SeqCst},
        },
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use actix_http::Response;
use log::{debug, info};
use serde::Deserialize;

mod gsi;
mod http_error;

use actix_web::{
    delete,
    error::{ErrorForbidden, ErrorNotImplemented, ErrorTooManyRequests, ErrorUnauthorized},
    get, post, web, Responder, Result,
};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use http_error::IntoHttpError;

use crate::{
    data_struct::{SecretMessage, Subscription, User},
    db,
    notifier::{self, AppPushMessage},
};

pub struct AppState {
    pub db: db::DB,
    pub web_push: Arc<notifier::WebPusher>,
    pub block_registration: bool,
    pub scheduled_task_period: u64,
    pub oauth_client_id: String,
    pub serverless_token: String,
    pub scheduled_task_running: AtomicBool,
}

async fn authorize_user(access_token: &str, data: &web::Data<AppState>) -> Result<String> {
    // NOTE: auth user is expected to return httpResponse error on failure
    // http_error currently sets the status code correctly but ignores the message
    // part

    let email = gsi::get_email_from_token(access_token, &data.oauth_client_id)
        .await
        .http_unauthorized_error("cannot get valid email from token")?;

    debug!("authorizing {}", email);
    let res =
        data.db.get_user(email.as_str()).await.http_unauthorized_error("email is not registered");
    info!("done get {}", email);

    // exit if user doesn't exist and new registration isn't allowed
    if data.block_registration {
        if let Err(e) = res {
            debug!(
                "rejecting auth request from {}, email is not registered and registration disabled",
                email
            );
            return Err(e);
        }
    }

    // update last seen
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let sub = if let Ok(user) = res { user.subscription } else { Subscription::default() };

    data.db
        .put_user(User { id: email.clone(), last_seen: now, subscription: sub })
        .await
        .http_internal_error(&format!("cannot update last_seen for user {}", email))?;

    info!("{} authorized and updated", email);
    Ok(email)
}

#[get("/serverless-task")]
async fn serverless_scheduled_task(
    data: web::Data<AppState>,
    auth: BearerAuth,
) -> Result<impl Responder> {
    if data.serverless_token.is_empty() {
        return Err(ErrorNotImplemented("this feature is not active\n"));
    }
    if auth.token() != data.serverless_token {
        return Err(ErrorUnauthorized("correct access token required\n"));
    }
    if data.scheduled_task_running.load(std::sync::atomic::Ordering::Acquire) {
        return Err(ErrorTooManyRequests("task is still executing\n"));
    }
    if data.scheduled_task_running.compare_exchange(false, true, SeqCst, Acquire).is_err() {
        return Err(ErrorTooManyRequests("task is still executing\n"));
    }
    notifier::execute_tasks(&data.db, &data.web_push)
        .await
        .http_internal_error("error executing scheduled task")?;
    Ok("task executed successfully\n")
}

#[get("/message-list")]
async fn message_list(data: web::Data<AppState>, auth: BearerAuth) -> Result<impl Responder> {
    let email = authorize_user(auth.token(), &data).await?;
    info!("getting message for {}", email);
    let messages = data
        .db
        .get_messages_for_email(email)
        .await
        .map_err(|e| string_error::into_err(e.to_string()))?;
    Ok(web::Json(messages))
}

#[get("/user-pong")]
async fn user_pong(data: web::Data<AppState>, auth: BearerAuth) -> Result<impl Responder> {
    let email = authorize_user(auth.token(), &data).await?;
    info!("received pong from {}", email);
    Ok(Response::ok())
}

#[derive(Deserialize, Default)]
struct TestNotificationRequest {
    #[serde(default)]
    recipient: String,
}

#[post("/test-notification")]
async fn test_notification(
    data: web::Data<AppState>,
    notif_request: web::Json<TestNotificationRequest>,
    auth: BearerAuth,
) -> Result<impl Responder> {
    let email = authorize_user(auth.token(), &data).await?;
    let recipient_email = if !notif_request.recipient.is_empty() {
        notif_request.recipient.as_str()
    } else {
        email.as_str()
    };
    let user = data
        .db
        .get_user(recipient_email)
        .await
        .map_err(|e| string_error::into_err(e.to_string()))?;

    let mut push_message = AppPushMessage { tag: "test".to_string(), ..Default::default() };
    if user.id == email {
        push_message.title = "Ywinby says 👋".to_string();
        push_message.message =
            "This means you're ready to receive future notifications!".to_string();
    } else {
        push_message.title = email.clone() + " says 👋";
        push_message.message = email.clone() + " wants to confirm that you're active on Ywinby";
    }
    data.web_push
        .send_message(user.subscription, push_message)
        .await
        .http_internal_error("cannot send push message")?;
    info!("push notification test message sent to {}", user.id);
    Ok(Response::ok())
}
#[post("/unsubscribe-user")]
async fn unsubscribe_user(data: web::Data<AppState>, auth: BearerAuth) -> Result<impl Responder> {
    let email = authorize_user(auth.token(), &data).await?;
    data.db
        .unsubscribe_user(email.to_owned())
        .await
        .map_err(|e| string_error::into_err(e.to_string()))?;
    info!("{} unsubscribed", email);
    Ok(Response::ok())
}

#[derive(Deserialize)]
pub struct SubscriptionRequest {
    subscription: Subscription,
}

#[post("/subscribe-user")]
async fn subscribe_user(
    data: web::Data<AppState>,
    auth: BearerAuth,
    req: web::Json<SubscriptionRequest>,
) -> Result<impl Responder> {
    let email = authorize_user(auth.token(), &data).await?;
    data.db
        .subscribe_user(email.clone(), req.subscription.clone())
        .await
        .map_err(|e| string_error::into_err(e.to_string()))?;
    info!("{} subscribed", email);
    Ok(Response::ok())
}

#[derive(Deserialize)]
struct DeleteMessage {
    message_id: String,
}

#[delete("/message")]
async fn message_delete(
    data: web::Data<AppState>,
    del_msg: web::Json<DeleteMessage>,
    auth: BearerAuth,
) -> Result<impl Responder> {
    let email = authorize_user(auth.token(), &data).await?;
    data.db
        .delete_message_from_email(email.to_owned(), del_msg.message_id.clone())
        .await
        .map_err(|e| string_error::into_err(e.to_string()))?;
    info!("{} deleted message {}", email, del_msg.message_id);
    Ok(Response::ok())
}

#[derive(Deserialize)]
struct NewMessage {
    message: SecretMessage,
}
#[post("/message")]
async fn message_create(
    data: web::Data<AppState>,
    auth: BearerAuth,
    new_message: web::Json<NewMessage>,
) -> Result<impl Responder> {
    let email = authorize_user(auth.token(), &data).await?;
    let mut m: SecretMessage = new_message.into_inner().message;
    info!(
        "scheduled_task_minute: {} every_minute: {}",
        data.scheduled_task_period / 60,
        m.verify_every_minutes
    );
    if data.scheduled_task_period > m.verify_every_minutes * 60 {
        return Err(ErrorForbidden(format!(
            "verification time is too short, server minimum is {:.0} minutes",
            data.scheduled_task_period / 60
        )));
    }
    let recipient = data
        .db
        .get_user(m.recipient.as_str())
        .await
        .http_not_found_error("recipient email is not registered")?;
    if recipient.id == email {
        return Err(ErrorForbidden("owner and recipient must be different"));
    }
    if recipient.subscription.keys.auth.is_empty() {
        return Err(ErrorForbidden("recipient hasn't subscribe to push notification"));
    }
    m.owner = email.to_owned();
    data.db.put_message(m).await.map_err(|e| string_error::into_err(e.to_string()))?;
    info!("{} created message for {})", email, recipient.id);
    Ok(Response::ok())
}
