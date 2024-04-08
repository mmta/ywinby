use std::collections::BTreeMap;

pub mod firestore;
pub mod jfs_store;

use crate::data_struct::{MessageWithLastSeen, SecretMessage, Subscription, User};

type DBResult<T> = anyhow::Result<T>;

pub enum DB {
    Firestore { storage: firestore::Storage },
    Json { storage: jfs_store::Storage },
}

impl DB {
    pub async fn put_user(&self, user: User) -> DBResult<()> {
        match self {
            DB::Firestore { storage } => storage.put_user(user).await,
            DB::Json { storage } => storage.put_user(user).await,
        }
    }
    pub async fn get_user(&self, id: &str) -> DBResult<User> {
        match self {
            DB::Firestore { storage } => storage.get_user(id).await,
            DB::Json { storage } => storage.get_user(id).await,
        }
    }
    pub async fn put_message(&self, message: SecretMessage) -> DBResult<()> {
        match self {
            DB::Firestore { storage } => storage.put_message(message).await,
            DB::Json { storage } => storage.put_message(message).await,
        }
    }
    pub async fn update_message_notified_on(&self, id: &str, email: &str) -> DBResult<()> {
        match self {
            DB::Firestore { storage } => storage.update_message_notified_on(id, email).await,
            DB::Json { storage } => storage.update_message_notified_on(id, email).await,
        }
    }
    pub async fn set_message_revealed_if_needed(&self, id: &str) -> DBResult<bool> {
        match self {
            DB::Firestore { storage } => storage.set_message_revealed_if_needed(id).await,
            DB::Json { storage } => storage.set_message_revealed_if_needed(id).await,
        }
    }
    pub async fn get_messages_for_email(
        &self,
        email: String,
    ) -> DBResult<Vec<MessageWithLastSeen>> {
        match self {
            DB::Firestore { storage } => storage.get_messages_for_email(email).await,
            DB::Json { storage } => storage.get_messages_for_email(email).await,
        }
    }
    pub async fn delete_message_from_email(
        &self,
        email: String,
        message_id: String,
    ) -> DBResult<()> {
        match self {
            DB::Firestore { storage } => storage.delete_message_from_email(email, message_id).await,
            DB::Json { storage } => storage.delete_message_from_email(email, message_id).await,
        }
    }
    pub async fn get_all_messages(&self) -> DBResult<BTreeMap<String, SecretMessage>> {
        match self {
            DB::Firestore { storage } => storage.get_all_messages().await,
            DB::Json { storage } => storage.get_all_messages().await,
        }
    }
    pub async fn unsubscribe_user(&self, email: String) -> DBResult<()> {
        match self {
            DB::Firestore { storage } => storage.unsubscribe_user(email).await,
            DB::Json { storage } => storage.unsubscribe_user(email).await,
        }
    }
    pub async fn subscribe_user(&self, email: String, sub: Subscription) -> DBResult<()> {
        match self {
            DB::Firestore { storage } => storage.subscribe_user(email, sub).await,
            DB::Json { storage } => storage.subscribe_user(email, sub).await,
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
