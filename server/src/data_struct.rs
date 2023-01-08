use serde::{ Deserialize, Serialize };
use std::time::{ SystemTime, UNIX_EPOCH, SystemTimeError };

pub type UserID = String;

const MINIMUM_SECONDS_BETWEEN_RECIPIENT_NOTIFICATION: u64 = 86400; // 24 hrs

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct User {
  pub id: UserID,
  pub last_seen: u64,
  #[serde(default)]
  pub subscription: Subscription,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Subscription {
  pub endpoint: String,
  pub keys: Keys,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Keys {
  pub p256dh: String,
  pub auth: String,
}

#[derive(Serialize, Default)]
pub struct MessageWithLastSeen {
  pub recipient: UserID,
  pub system_share: String,
  pub verify_every_minutes: u64,
  pub max_failed_verification: u64,
  pub owner: UserID,
  pub created_ts: u64,
  pub revealed: bool,
  pub id: String,
  pub owner_last_seen: u64,
  pub recipient_last_seen: u64,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct SecretMessage {
  pub recipient: UserID,
  pub system_share: String,
  pub verify_every_minutes: u64,
  pub max_failed_verification: u64,
  #[serde(default)]
  pub owner: UserID,
  #[serde(default)]
  pub created_ts: u64,
  #[serde(default)]
  pub recipient_notified_on: u64,
  #[serde(default)]
  pub owner_notified_on: u64,
  #[serde(default)]
  pub revealed: bool,
  #[serde(default)]
  pub id: String,
}

impl SecretMessage {
  pub fn should_reveal(&self, owner_last_seen: u64) -> Result<bool, SystemTimeError> {
    if self.revealed {
      return Ok(true);
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let max_allowed = self.verify_every_minutes * 60 * self.max_failed_verification;
    let reveal_time = owner_last_seen + max_allowed;
    Ok(now >= reveal_time)
  }

  pub fn should_notify_recipient(&self, owner_last_seen: u64) -> Result<bool, SystemTimeError> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let reveal = self.should_reveal(owner_last_seen)?;
    let notify_time = self.recipient_notified_on + MINIMUM_SECONDS_BETWEEN_RECIPIENT_NOTIFICATION;
    Ok(reveal && now >= notify_time)
  }

  pub fn should_notify_owner(&self, owner_last_seen: u64) -> Result<bool, SystemTimeError> {
    if self.revealed {
      return Ok(false);
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let newer = if owner_last_seen > self.owner_notified_on {
      owner_last_seen
    } else {
      self.owner_notified_on
    };
    let notify_time = newer + self.verify_every_minutes * 60;
    Ok(now >= notify_time)
  }
}
