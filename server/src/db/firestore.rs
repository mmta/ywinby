use std::collections::BTreeMap;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use crate::data_struct::SecretMessage;
use crate::data_struct::User;
use crossbeam::channel;
use log::info;
use uuid::Uuid;

use super::StorageType;
use super::{ DB, DBResult };
use crate::data_struct::MessageWithLastSeen;
use crate::data_struct::Subscription;

use anyhow::{ anyhow, format_err };

use firestore::*;

pub struct Storage {
  db: FirestoreDb,
  user_coll: String,
  message_coll: String,
  rt: tokio::runtime::Runtime,
  project_id: String,
}

impl Storage {
  pub async fn new(project_id: &str) -> DBResult<Storage> {
    let fdb = FirestoreDb::new(project_id).await?;
    Ok(Storage {
      db: fdb,
      user_coll: "users".to_string(),
      message_coll: "messages".to_string(),
      rt: tokio::runtime::Runtime::new().unwrap(),
      project_id: project_id.to_string(),
    })
  }
}

impl DB for Storage {
  fn get_storage(&self) -> DBResult<(StorageType, String)> {
    Ok((StorageType::Firestore, self.project_id.to_string()))
  }
  fn put_user(&self, user: User) -> DBResult<()> {
    let (tx, rx) = channel::bounded(1);
    let db = self.db.clone();
    let uid = user.id.clone();
    let u = user.clone();
    let coll = self.user_coll.clone();

    self.rt.spawn(async move {
      let res: Result<User, errors::FirestoreError> = db
        .fluent()
        .update()
        .in_col(&coll)
        .document_id(&uid)
        .object(&u)
        .execute().await;
      if res.is_ok() {
        _ = tx.send(res);
        return;
      }
      let err = res.unwrap_err();
      if !err.to_string().contains("NotFound") {
        _ = tx.send(Err(err));
      }
      let res: Result<User, errors::FirestoreError> = db
        .fluent()
        .insert()
        .into(&coll)
        .document_id(&uid)
        .object(&u)
        .execute().await;
      _ = tx.send(res);
    });
    _ = rx.recv()??;
    info!("user upserted, Id: {}", user.id);
    Ok(())
  }
  fn get_user(&self, id: &str) -> DBResult<User> {
    let (tx, rx) = channel::bounded(1);
    let db = self.db.clone();
    let i = id.to_owned();
    let coll = self.user_coll.clone();
    self.rt.spawn(async move {
      let v = db.fluent().select().by_id_in(&coll).obj().one(i).await;
      _ = tx.send(v);
    });
    let m = rx.recv()??;
    m.ok_or_else(|| format_err!("cannot find user"))
  }
  fn put_message(&self, mut message: SecretMessage) -> DBResult<()> {
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
    let id = &Uuid::new_v4().to_string();
    let m_ida = id.clone();
    let m_idb = id.clone();

    let (tx, rx) = channel::bounded(1);
    let db = self.db.clone();
    let mut m = message;
    m.id = id.to_owned();
    let coll = self.message_coll.clone();

    self.rt.spawn(async move {
      let res: Result<SecretMessage, errors::FirestoreError> = db
        .fluent()
        .insert()
        .into(&coll)
        .document_id(m_ida.to_owned())
        .object(&m)
        .execute().await;
      _ = tx.send(res);
    });
    _ = rx.recv()??;
    info!("message upserted, Id: {}", m_idb);
    Ok(())
  }
  fn update_message_notified_on(&self, id: &str, email: &str) -> DBResult<()> {
    let mut message: SecretMessage = self.get_message(id)?;
    if let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) {
      if email == message.recipient {
        message.recipient_notified_on = now.as_secs();
      } else if email == message.owner {
        message.owner_notified_on = now.as_secs();
      }
      let (tx, rx) = channel::bounded(1);
      let db = self.db.clone();
      let coll = self.message_coll.clone();
      let i = id.to_string();
      self.rt.spawn(async move {
        let res: Result<SecretMessage, _> = db
          .fluent()
          .update()
          .in_col(&coll)
          .document_id(&i)
          .object(&message)
          .execute().await;
        _ = tx.send(res);
      });
      _ = rx.recv()??;
    }
    Ok(())
  }

  fn set_message_revealed_if_needed(&self, id: &str) -> DBResult<bool> {
    let mut m = self.get_message(id)?;
    if m.revealed {
      return Ok(true);
    }
    // not revealed yet in db
    let owner = self.get_user(&m.owner)?;
    if let Ok(r) = m.should_reveal(owner.last_seen) {
      if r {
        m.revealed = true;
        let (tx, rx) = channel::bounded(1);
        let db = self.db.clone();
        let coll = self.message_coll.clone();
        let i = id.to_string();
        self.rt.spawn(async move {
          let res: Result<SecretMessage, _> = db
            .fluent()
            .update()
            .in_col(&coll)
            .document_id(&i)
            .object(&m)
            .execute().await;
          _ = tx.send(res);
        });
        _ = rx.recv()??;
        return Ok(true);
      }
    }
    Ok(false)
  }
  fn get_message(&self, id: &str) -> DBResult<SecretMessage> {
    let (tx, rx) = channel::bounded(1);
    let db = self.db.clone();
    let coll = self.message_coll.clone();
    let i = id.to_string();
    self.rt.spawn(async move {
      let res = db.fluent().select().by_id_in(&coll).obj().one(i).await;
      _ = tx.send(res);
    });
    let m = rx.recv()??;
    m.ok_or_else(|| format_err!("cannot find message"))
  }
  fn get_messages_for_email(&self, email: String) -> DBResult<Vec<MessageWithLastSeen>> {
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

  fn delete_message_from_email(&self, email: String, message_id: String) -> DBResult<()> {
    let messages: BTreeMap<String, SecretMessage> = self
      .get_all_messages()?
      .into_iter()
      .filter(|x| (x.1.owner == email || x.1.recipient == email))
      .collect();

    if messages.contains_key(message_id.as_str()) {
      let message: SecretMessage = self.get_message(&message_id)?;
      let should_delete = if email == message.recipient {
        self.set_message_revealed_if_needed(message_id.as_str())?
      } else {
        email == message.owner
      };
      if should_delete {
        let (tx, rx) = channel::bounded(1);
        let db = self.db.clone();
        let coll = self.message_coll.clone();
        self.rt.spawn(async move {
          let res = db.fluent().delete().from(&coll).document_id(&message_id).execute().await;
          _ = tx.send(res);
        });
        rx.recv()??;
        return Ok(());
      }
    }
    Err(anyhow!("message not found"))
  }

  fn get_all_messages(&self) -> DBResult<BTreeMap<String, SecretMessage>> {
    let (tx, rx) = channel::bounded(1);
    let db = self.db.clone();
    let coll = self.message_coll.clone();
    self.rt.spawn(async move {
      let res: Result<Vec<SecretMessage>, _> = db
        .fluent()
        .select()
        .from(coll.as_str())
        .obj()
        .query().await;
      _ = tx.send(res);
    });
    let coll = rx.recv()??;

    let mut res: BTreeMap<String, SecretMessage> = BTreeMap::new();
    for sm in coll {
      res.insert(sm.id.clone(), sm);
    }
    Ok(res)
  }
  fn unsubscribe_user(&self, email: String) -> DBResult<()> {
    let user: User = self.get_user(&email)?;
    let new_user = User {
      id: user.id.clone(),
      last_seen: user.last_seen,
      ..Default::default()
    };
    self.put_user(new_user)?;
    Ok(())
  }
  fn subscribe_user(&self, email: String, sub: Subscription) -> DBResult<()> {
    let user: User = self.get_user(&email)?;
    let new_user = User { id: user.id.clone(), last_seen: user.last_seen, subscription: sub };
    self.put_user(new_user)?;
    Ok(())
  }
}
