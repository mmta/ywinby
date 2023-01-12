use std::collections::BTreeMap;
use std::fs::create_dir_all;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use crate::data_struct::SecretMessage;
use crate::data_struct::User;
use anyhow::anyhow;
use jfs::Store;
use log::info;

use super::DBResult;
use crate::data_struct::MessageWithLastSeen;
use crate::data_struct::Subscription;

pub struct Storage {
  user_store: Store,
  message_store: Store,
  pub mu: Arc<Mutex<()>>,
}

impl Storage {
  pub fn new(id: &str) -> DBResult<Storage> {
    let cfg = jfs::Config { pretty: true, single: true, ..Default::default() };
    let db_path = Path::new(id);
    create_dir_all(db_path)?;
    let u = Store::new_with_cfg(db_path.join("users").as_path(), cfg)?;
    let m = Store::new_with_cfg(db_path.join("messages").as_path(), cfg)?;
    Ok(Storage {
      user_store: u,
      message_store: m,
      mu: Arc::new(Mutex::new(())),
    })
  }
  pub fn put_user(&self, user: User) -> DBResult<()> {
    let id = self.user_store.save_with_id(&user, &user.id)?;
    info!("user upserted, Id: {}", id);
    Ok(())
  }
  pub fn get_user(&self, id: &str) -> DBResult<User> {
    let u = self.user_store.get::<User>(id)?;
    Ok(u)
  }
  pub fn put_message(&self, mut message: SecretMessage) -> DBResult<()> {
    if message.owner.is_empty() {
      return Err(anyhow!("owner must not be empty"));
    }
    if message.max_failed_verification < 1 || message.max_failed_verification > 9 {
      return Err(anyhow!("maximum consecutive failure should be between 1 and 9"));
    }
    if message.verify_every_minutes < 1 || message.verify_every_minutes > 4336204 {
      return Err(
        anyhow!("maximum time between verification should be between 1 minute and 99 months")
      );
    }
    if let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) {
      message.created_ts = now.as_secs();
    } else {
      return Err(anyhow!("cannot set message creation timestamp"));
    }

    let id = self.message_store.save(&message)?;
    info!("message upserted, Id: {}", id);
    Ok(())
  }
  pub fn update_message_notified_on(&self, id: &str, email: &str) -> DBResult<()> {
    let mut message: SecretMessage = self.message_store.get(id)?;
    if let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) {
      if email == message.recipient {
        message.recipient_notified_on = now.as_secs();
      } else if email == message.owner {
        message.owner_notified_on = now.as_secs();
      }
      self.message_store.save_with_id(&message, id)?;
    }
    Ok(())
  }

  pub fn set_message_revealed_if_needed(&self, id: &str) -> DBResult<bool> {
    let mut m = self.get_message(id)?;
    if m.revealed {
      return Ok(true);
    }
    // not revealed yet in db
    let owner = self.get_user(&m.owner)?;
    if let Ok(r) = m.should_reveal(owner.last_seen) {
      if r {
        m.revealed = true;
        self.message_store.save_with_id(&m, id)?;
        return Ok(true);
      }
    }
    Ok(false)
  }
  fn get_message(&self, id: &str) -> DBResult<SecretMessage> {
    let message = self.message_store.get(id)?;
    Ok(message)
  }
  pub fn get_messages_for_email(&self, email: String) -> DBResult<Vec<MessageWithLastSeen>> {
    let messages: BTreeMap<String, SecretMessage> = self
      .get_all_messages()?
      .into_iter()
      .filter(|x| (x.1.owner == email || x.1.recipient == email))
      .collect();

    let mut out: Vec<MessageWithLastSeen> = Vec::new();
    for (k, v) in messages {
      let owner = self.get_user(&v.owner)?;
      let recipient = self.get_user(&v.recipient)?;
      let mut m = MessageWithLastSeen {
        id: k.to_owned(),
        created_ts: v.created_ts,
        owner: owner.id,
        recipient: recipient.id,
        system_share: "".to_owned(),
        verify_every_minutes: v.verify_every_minutes,
        max_failed_verification: v.max_failed_verification,
        owner_last_seen: owner.last_seen,
        recipient_last_seen: recipient.last_seen,
        revealed: v.revealed,
      };

      // first set revealed on db if needed, this flag should only change from false -> true once
      m.revealed = self.set_message_revealed_if_needed(&k)?;

      if email == v.owner {
        m.system_share = v.system_share.clone();
      }
      // disclose system share to recipient if revealed is true
      if email == v.recipient && m.revealed {
        m.system_share = v.system_share.clone();
      }
      out.push(m);
    }
    Ok(out)
  }

  pub fn delete_message_from_email(&self, email: String, message_id: String) -> DBResult<()> {
    let messages: BTreeMap<String, SecretMessage> = self
      .get_all_messages()?
      .into_iter()
      .filter(|x| (x.1.owner == email || x.1.recipient == email))
      .collect();

    if messages.contains_key(message_id.as_str()) {
      let message: SecretMessage = self.message_store.get(&message_id)?;
      let should_delete = if email == message.recipient {
        self.set_message_revealed_if_needed(message_id.as_str())?
      } else {
        email == message.owner
      };
      if should_delete {
        self.message_store.delete(message_id.as_str())?;
        return Ok(());
      }
    }
    Err(anyhow!("message not found"))
  }
  pub fn get_all_messages(&self) -> DBResult<BTreeMap<String, SecretMessage>> {
    let res = self.message_store.all::<SecretMessage>()?;
    Ok(res)
  }
  pub fn unsubscribe_user(&self, email: String) -> DBResult<()> {
    let user: User = self.user_store.get(email.as_str())?;
    let new_user = User {
      id: user.id.clone(),
      last_seen: user.last_seen,
      ..Default::default()
    };
    self.user_store.delete(&user.id)?;
    self.put_user(new_user)?;
    Ok(())
  }
  pub fn subscribe_user(&self, email: String, sub: Subscription) -> DBResult<()> {
    let user: User = self.user_store.get(email.as_str())?;
    let new_user = User { id: user.id.clone(), last_seen: user.last_seen, subscription: sub };
    self.user_store.delete(&user.id)?;
    self.put_user(new_user)?;
    Ok(())
  }
}
