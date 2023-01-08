use anyhow::Result;
use crossbeam::{ channel::{ Receiver, tick }, select };
use serde::Serialize;
use web_push::*;
use log::{ info, error };
use std::{ collections::HashSet, time::Duration };

use crate::{ data_struct::{ User, SecretMessage, Subscription }, db::DB };

const PUSH_SUBJECT_CLAIM: &str = "https://github.com/mmta/ywinby";
pub const SCHEDULER_CHECK_EXIT_EVERY_SECONDS: u64 = 10;

pub async fn start_scheduler(
  db: Box<dyn DB>,
  every_seconds: u64,
  quit: Receiver<bool>,
  webpush_privkey_base64: String
) {
  let res = WebPusher::new(webpush_privkey_base64);
  if res.is_err() {
    error!("cannot start scheduler, failed to initialize web push client");
    return;
  }
  let web_pusher = res.unwrap();

  info!("scheduler will execute task every {} seconds", every_seconds);
  let ticker = tick(Duration::from_secs(every_seconds));

  loop {
    select! {
      recv(ticker) -> _ => {
        let _res = execute_tasks(&db, &web_pusher).await
          .map_err(|e| error!("error executing task: {}", e))
          .unwrap_or_default();
      },
      // spawn() doesnt work with select!'ing quit channel, so here's a workaround
      default(Duration::from_secs(SCHEDULER_CHECK_EXIT_EVERY_SECONDS)) => {
        if let Ok(_) = quit.try_recv() {
          break
        }
      }
    }
  }
}

pub async fn execute_tasks(db: &Box<dyn DB>, pusher: &WebPusher) -> Result<()> {
  info!("start executing scheduled task");

  let messages = db.get_all_messages()?;
  let mut notifications: HashSet<Notification> = HashSet::new();

  for (k, v) in messages {
    let res = db.get_user(v.owner.as_str());
    if res.is_err() {
      error!("cannot get owner for {}, skip processing", k);
      continue;
    }
    let o = res.as_ref().unwrap();
    let res = db.get_user(v.recipient.as_str());
    if res.is_err() {
      error!("cannot get recipient for {}, skip processing", k);
      continue;
    }
    let r = res.as_ref().unwrap();
    if let Some(v) = get_notification(o.clone(), r.clone(), k.to_owned(), v) {
      notifications.insert(Notification {
        app_message: v.0,
        subscription: v.1.subscription,
        email: v.1.id,
        message_id: k,
      });
    }
  }
  for n in notifications {
    if let Err(e) = pusher.send_message(n.subscription.clone(), n.app_message).await {
      error!(
        "cannot push notification to {} about message {}: {}",
        n.email,
        n.message_id,
        e.to_string()
      );
      error!("subscription endpoint: {:?}", n.subscription.endpoint);
    } else {
      info!("push message sent for message Id: {}", n.message_id);
      if let Err(e) = db.update_message_notified_on(n.message_id.as_str(), &n.email) {
        error!("cannot set message last notification timestamp {}: {}", n.message_id, e);
      }
      if let Err(e) = db.set_message_revealed_if_needed(&n.message_id) {
        error!("cannot set message revealed flag {}: {}", n.message_id, e);
      }
    }
  }
  info!("done executing scheduled task");
  Ok(())
}

#[derive(Eq, Hash, PartialEq)]
struct Notification {
  email: String,
  message_id: String,
  app_message: AppPushMessage,
  subscription: Subscription,
}

fn get_notification(
  owner: User,
  recipient: User,
  k: String,
  v: SecretMessage
) -> Option<(AppPushMessage, User)> {
  info!("processing message Id: {}", k);

  // owner first, so they will receive the configured max number of notifications
  if let Ok(true) = v.should_notify_owner(owner.last_seen) {
    info!("notifying owner {}", owner.id);
    let msg = AppPushMessage {
      tag: "owner".to_owned(),
      title: "Owner verification".to_owned(),
      message: "Time to verify your presence!".to_owned(),
    };
    return Some((msg, owner));
  }

  // notify recipient on the next execute_task cycle, of which should_verify_owner will have
  // return false
  if let Ok(true) = v.should_notify_recipient(owner.last_seen) {
    info!("notifying recipient {}", recipient.id);

    let msg = AppPushMessage {
      tag: "recipient".to_owned(),
      title: "Secret message unlocked!".to_owned(),
      message: "You can now reveal the message from ".to_owned() +
      owner.id.as_str() +
      ". Please delete the message after that to stop this alert.",
    };
    return Some((msg, recipient));
  }
  info!("notification not sent for Id: {}", k);
  None
}

#[derive(Serialize, PartialEq, Eq, Hash, Default)]
pub struct AppPushMessage {
  pub tag: String,
  pub title: String,
  pub message: String,
}

#[derive(Clone)]
pub struct WebPusher {
  privkey_base64: String,
  client: WebPushClient,
}

impl WebPusher {
  pub fn new(privkey_base64: String) -> Result<Self, WebPushError> {
    let client = WebPushClient::new()?;
    Ok(Self { privkey_base64, client })
  }
  pub async fn send_message(&self, sub: Subscription, message: AppPushMessage) -> Result<()> {
    let subscription_info = SubscriptionInfo::new(sub.endpoint, sub.keys.p256dh, sub.keys.auth);
    let mut sig_builder = VapidSignatureBuilder::from_base64(
      self.privkey_base64.as_str(),
      web_push::URL_SAFE_NO_PAD,
      &subscription_info
    )?;
    sig_builder.add_claim("sub", PUSH_SUBJECT_CLAIM);
    let signature = sig_builder.build()?;

    let json = serde_json::to_string(&message)?;
    let content = json.as_bytes();
    let mut builder = WebPushMessageBuilder::new(&subscription_info)?;
    builder.set_payload(ContentEncoding::Aes128Gcm, content);
    builder.set_vapid_signature(signature);
    builder.set_ttl(1000);
    self.client.send(builder.build()?).await?;
    Ok(())
  }
}
