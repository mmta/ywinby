use actix_http::Response;
use serde::Deserialize;
use std::time::{ SystemTime, UNIX_EPOCH };
use log::{ info, debug };

mod http_error;
mod gsi;

use actix_web::{
  web,
  Responder,
  Result,
  post,
  delete,
  error::{ ErrorForbidden, ErrorUnauthorized, ErrorNotImplemented },
};
use http_error::IntoHttpError;

use crate::{ notifier::{ self, AppPushMessage }, data_struct::{ SecretMessage, User }, db::Unsafe };
use crate::data_struct::{ Subscription };

pub struct AppState {
  pub db: Box<dyn crate::db::DB>,
  pub unsafe_db: Unsafe,
  pub web_push: notifier::WebPusher,
  pub block_registration: bool,
  pub scheduled_task_period: u64,
  pub oauth_client_id: String,
  pub serverless_token: String,
}

async fn authenticate_user(access_token: &str, data: &web::Data<AppState>) -> Result<String> {
  // NOTE: authenticate user is expected to return httpResponse error on failure
  // http_error currently sets the status code correctly but ignores the message part

  let email = gsi
    ::get_email_from_token(access_token, &data.oauth_client_id).await
    .http_unauthorized_error("cannot get valid email from token")?;

  debug!("authenticating {}", email);
  let res = data.db.get_user(email.as_str()).http_unauthorized_error("email is not registered");
  info!("done get {}", email);

  // exit if user doesn't exist and new registration isn't allowed
  if res.is_err() && data.block_registration {
    debug!("rejecting auth request from {}, email is not registered and registration disabled", email);
    return Err(res.unwrap_err());
  }
  // update last seen
  let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
  let sub = if res.is_ok() { res.unwrap().subscription } else { Subscription::default() };
  data.db
    .put_user(User { id: email.clone(), last_seen: now, subscription: sub })
    .http_internal_error(format!("cannot update last_seen for user {}", email).as_str())?;

  info!("{} authenticated and updated", email);
  Ok(email)
}

#[post("/serverless-task")]
async fn serverless_scheduled_task(
  data: web::Data<AppState>,
  access_token: web::Json<AccessToken>
) -> Result<impl Responder> {
  if data.serverless_token.is_empty() {
    return Err(ErrorNotImplemented("this feature is not active\n"));
  }
  if access_token.token != data.serverless_token {
    return Err(ErrorUnauthorized("correct access token required\n"));
  }

  notifier
    ::execute_tasks(&data.unsafe_db, &data.web_push).await
    .http_internal_error("error executing scheduled task")?;
  Ok("task executed successfully\n")
}

#[derive(Deserialize)]
pub struct AccessToken {
  token: String,
}

#[post("/message-list")]
async fn message_list(
  data: web::Data<AppState>,
  access_token: web::Json<AccessToken>
) -> Result<impl Responder> {
  let email = authenticate_user(access_token.token.as_str(), &data).await?;
  info!("getting message for {}", email);
  let messages = data.db
    .get_messages_for_email(email)
    .map_err(|e| string_error::into_err(e.to_string()))?;
  Ok(web::Json(messages))
}

#[post("/user-pong")]
async fn user_pong(
  data: web::Data<AppState>,
  access_token: web::Json<AccessToken>
) -> Result<impl Responder> {
  let email = authenticate_user(access_token.token.as_str(), &data).await?;
  info!("received pong from {}", email);
  Ok(Response::ok())
}

#[derive(Deserialize, Default)]
struct TestNotificationRequest {
  token: String,
  #[serde(default)]
  recipient: String,
}

#[post("/test-notification")]
async fn test_notification(
  data: web::Data<AppState>,
  notif_request: web::Json<TestNotificationRequest>
) -> Result<impl Responder> {
  let email = authenticate_user(notif_request.token.as_str(), &data).await?;
  let recipient_email = if !notif_request.recipient.is_empty() {
    notif_request.recipient.as_str()
  } else {
    email.as_str()
  };
  let user = data.db.get_user(recipient_email).map_err(|e| string_error::into_err(e.to_string()))?;

  let mut push_message = AppPushMessage { tag: "test".to_string(), ..Default::default() };
  if user.id == email {
    push_message.title = "Ywinby says 👋".to_string();
    push_message.message = "This means you're ready to receive future notifications!".to_string();
  } else {
    push_message.title = email.clone() + " says 👋";
    push_message.message = email.clone() + " wants to confirm that you're active on Ywinby";
  }
  data.web_push
    .send_message(user.subscription, push_message).await
    .http_internal_error("cannot send push message")?;
  info!("push notification test message sent to {}", user.id);
  Ok(Response::ok())
}
#[post("/unsubscribe-user")]
async fn unsubscribe_user(
  data: web::Data<AppState>,
  access_token: web::Json<AccessToken>
) -> Result<impl Responder> {
  let email = authenticate_user(access_token.token.as_str(), &data).await?;
  data.db.unsubscribe_user(email.to_owned()).map_err(|e| string_error::into_err(e.to_string()))?;
  info!("{} unsubscribed", email);
  Ok(Response::ok())
}

#[derive(Deserialize)]
pub struct SubscriptionRequest {
  token: String,
  subscription: Subscription,
}

#[post("/subscribe-user")]
async fn subscribe_user(
  data: web::Data<AppState>,
  req: web::Json<SubscriptionRequest>
) -> Result<impl Responder> {
  let email = authenticate_user(req.token.as_str(), &data).await?;
  data.db
    .subscribe_user(email.clone(), req.subscription.clone())
    .map_err(|e| string_error::into_err(e.to_string()))?;
  info!("{} subscribed", email);
  Ok(Response::ok())
}

#[derive(Deserialize)]
struct DeleteMessage {
  token: String,
  message_id: String,
}

#[delete("/message")]
async fn message_delete(
  data: web::Data<AppState>,
  del_msg: web::Json<DeleteMessage>
) -> Result<impl Responder> {
  let email = authenticate_user(del_msg.token.as_str(), &data).await?;
  data.db
    .delete_message_from_email(email.to_owned(), del_msg.message_id.clone())
    .map_err(|e| string_error::into_err(e.to_string()))?;
  info!("{} deleted message {}", email, del_msg.message_id);
  Ok(Response::ok())
}

#[derive(Deserialize)]
struct NewMessage {
  token: String,
  message: SecretMessage,
}
#[post("/message")]
async fn message_create(
  data: web::Data<AppState>,
  new_message: web::Json<NewMessage>
) -> Result<impl Responder> {
  let email = authenticate_user(new_message.token.as_str(), &data).await?;
  let mut m: SecretMessage = new_message.into_inner().message;
  info!(
    "scheduled_task_minute: {} every_minute: {}",
    data.scheduled_task_period / 60,
    m.verify_every_minutes
  );
  if data.scheduled_task_period > m.verify_every_minutes * 60 {
    return Err(
      ErrorForbidden(
        format!(
          "verification time is too short, server minimum is {:.0} minutes",
          data.scheduled_task_period / 60
        )
      )
    );
  }
  let recipient = data.db
    .get_user(m.recipient.as_str())
    .http_not_found_error("recipient email is not registered")?;
  if recipient.id == email {
    return Err(ErrorForbidden("owner and recipient must be different"));
  }
  if recipient.subscription.keys.auth.is_empty() {
    return Err(ErrorForbidden("recipient hasn't subscribe to push notification"));
  }
  m.owner = email.to_owned();
  data.db.put_message(m).map_err(|e| string_error::into_err(e.to_string()))?;
  info!("{} created message for {})", email, recipient.id);
  Ok(Response::ok())
}
