use std::collections::BTreeMap;

pub mod jfs_store;
pub mod firestore;

use crate::data_struct::{ User, SecretMessage, MessageWithLastSeen, Subscription };

type DBResult<T> = anyhow::Result<T>;

pub trait DB: Send + Sync {
  fn put_user(&self, user: User) -> DBResult<()>;
  fn get_user(&self, id: &str) -> DBResult<User>;
  fn put_message(&self, message: SecretMessage) -> DBResult<()>;
  fn update_message_notified_on(&self, id: &str, email: &str) -> DBResult<()>;
  fn set_message_revealed_if_needed(&self, id: &str) -> DBResult<bool>;
  fn get_message(&self, id: &str) -> DBResult<SecretMessage>;
  fn get_messages_for_email(&self, email: String) -> DBResult<Vec<MessageWithLastSeen>>;
  fn delete_message_from_email(&self, email: String, message_id: String) -> DBResult<()>;
  fn get_all_messages(&self) -> DBResult<BTreeMap<String, SecretMessage>>;
  fn unsubscribe_user(&self, email: String) -> DBResult<()>;
  fn subscribe_user(&self, email: String, sub: Subscription) -> DBResult<()>;
}

#[derive(clap::ValueEnum, Clone, PartialEq, Copy, Debug)]
pub enum StorageType {
  FIRESTORE,
  JSON,
}

pub struct DBBuilder {}

impl DBBuilder {
  pub async fn new(storage_type: StorageType, id: &str) -> DBResult<Box<dyn DB>> {
    if storage_type == StorageType::JSON {
      let j = jfs_store::Storage::new(id)?;
      return Ok(Box::new(j));
    } else {
      let f = firestore::Storage::new(id).await?;
      return Ok(Box::new(f));
    }
  }
}
