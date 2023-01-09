use std::collections::BTreeMap;

pub mod jfs_store;
pub mod firestore;

use crate::data_struct::{ User, SecretMessage, MessageWithLastSeen, Subscription };

type DBResult<T> = anyhow::Result<T>;

pub trait DB {
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
  fn get_storage(&self) -> DBResult<(StorageType, String)>;
}

#[derive(clap::ValueEnum, Clone, PartialEq, Copy, Debug)]
pub enum StorageType {
  Firestore,
  Json,
}

pub struct DBBuilder {}

impl DBBuilder {
  #[allow(clippy::new_ret_no_self)]
  pub async fn new(storage_type: StorageType, id: &str) -> DBResult<Box<dyn DB>> {
    if storage_type == StorageType::Json {
      let j = jfs_store::Storage::new(id)?;
      Ok(Box::new(j))
    } else {
      let f = firestore::Storage::new(id).await?;
      Ok(Box::new(f))
    }
  }
}

pub struct Unsafe {
  pub content: Box<dyn DB>,
}
unsafe impl Send for Unsafe {}
unsafe impl Sync for Unsafe {}

impl Unsafe {
  pub async fn new(storage_type: StorageType, id: String) -> DBResult<Self> {
    let dbo = DBBuilder::new(storage_type, &id).await?;
    Ok(Self { content: dbo })
  }
}
