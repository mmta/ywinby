use std::{ collections::BTreeMap, sync::{ Mutex, Arc } };

pub mod jfs_store;
pub mod firestore;

use crate::data_struct::{ User, SecretMessage, MessageWithLastSeen, Subscription };

type DBResult<T> = anyhow::Result<T>;

pub enum DB {
  Firestore {
    storage: firestore::Storage,
  },
  Json {
    storage: jfs_store::Storage,
  },
}

impl DB {
  pub fn get_lock(&self) -> &Arc<Mutex<()>> {
    match self {
      DB::Firestore { storage } => &storage.mu,
      DB::Json { storage } => &storage.mu,
    }
  }
  pub fn put_user(&self, user: User) -> DBResult<()> {
    match self {
      DB::Firestore { storage } => storage.put_user(user),
      DB::Json { storage } => storage.put_user(user),
    }
  }
  pub fn get_user(&self, id: &str) -> DBResult<User> {
    match self {
      DB::Firestore { storage } => storage.get_user(id),
      DB::Json { storage } => storage.get_user(id),
    }
  }
  pub fn put_message(&self, message: SecretMessage) -> DBResult<()> {
    match self {
      DB::Firestore { storage } => storage.put_message(message),
      DB::Json { storage } => storage.put_message(message),
    }
  }
  pub fn update_message_notified_on(&self, id: &str, email: &str) -> DBResult<()> {
    match self {
      DB::Firestore { storage } => storage.update_message_notified_on(id, email),
      DB::Json { storage } => storage.update_message_notified_on(id, email),
    }
  }
  pub fn set_message_revealed_if_needed(&self, id: &str) -> DBResult<bool> {
    match self {
      DB::Firestore { storage } => storage.set_message_revealed_if_needed(id),
      DB::Json { storage } => storage.set_message_revealed_if_needed(id),
    }
  }
  pub fn get_messages_for_email(&self, email: String) -> DBResult<Vec<MessageWithLastSeen>> {
    match self {
      DB::Firestore { storage } => storage.get_messages_for_email(email),
      DB::Json { storage } => storage.get_messages_for_email(email),
    }
  }
  pub fn delete_message_from_email(&self, email: String, message_id: String) -> DBResult<()> {
    match self {
      DB::Firestore { storage } => storage.delete_message_from_email(email, message_id),
      DB::Json { storage } => storage.delete_message_from_email(email, message_id),
    }
  }
  pub fn get_all_messages(&self) -> DBResult<BTreeMap<String, SecretMessage>> {
    match self {
      DB::Firestore { storage } => storage.get_all_messages(),
      DB::Json { storage } => storage.get_all_messages(),
    }
  }
  pub fn unsubscribe_user(&self, email: String) -> DBResult<()> {
    match self {
      DB::Firestore { storage } => storage.unsubscribe_user(email),
      DB::Json { storage } => storage.unsubscribe_user(email),
    }
  }
  pub fn subscribe_user(&self, email: String, sub: Subscription) -> DBResult<()> {
    match self {
      DB::Firestore { storage } => storage.subscribe_user(email, sub),
      DB::Json { storage } => storage.subscribe_user(email, sub),
    }
  }
}

#[derive(clap::ValueEnum, Clone, PartialEq, Copy, Debug)]
pub enum StorageType {
  Firestore,
  Json,
}

pub struct DBBuilder {}

impl DBBuilder {
  #[allow(clippy::new_ret_no_self)]
  pub async fn new(storage_type: StorageType, id: &str) -> DBResult<DB> {
    if storage_type == StorageType::Json {
      let j = jfs_store::Storage::new(id)?;
      Ok(DB::Json { storage: j })
    } else {
      let f = firestore::Storage::new(id).await?;
      Ok(DB::Firestore { storage: f })
    }
  }
}
