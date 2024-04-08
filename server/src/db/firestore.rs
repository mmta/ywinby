use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, format_err};
use firestore::*;
use log::info;
use uuid::Uuid;

use super::DBResult;
use crate::data_struct::{MessageWithLastSeen, SecretMessage, Subscription, User};

pub struct Storage {
    db: FirestoreDb,
    user_coll: String,
    message_coll: String,
}

impl Storage {
    pub async fn new(project_id: &str) -> DBResult<Storage> {
        let fdb = FirestoreDb::new(project_id).await?;
        Ok(Storage {
            db: fdb,
            user_coll: "users".to_string(),
            message_coll: "messages".to_string(),
        })
    }
    pub async fn put_user(&self, user: User) -> DBResult<()> {
        let res: Result<User, errors::FirestoreError> = self
            .db
            .fluent()
            .update()
            .in_col(&self.user_coll)
            .document_id(&user.id)
            .object(&user)
            .execute()
            .await;
        if res.is_ok() {
            return Ok(());
        }
        let err = res.unwrap_err();
        if !err.to_string().contains("NotFound") {
            return Err(err.into());
        }
        self.db
            .fluent()
            .insert()
            .into(&self.user_coll)
            .document_id(&user.id)
            .object(&user)
            .execute()
            .await?;

        info!("user upserted, Id: {}", user.id);
        Ok(())
    }
    pub async fn get_user(&self, id: &str) -> DBResult<User> {
        let m = self.db.fluent().select().by_id_in(&self.user_coll).obj().one(id).await?;
        m.ok_or_else(|| format_err!("cannot find user"))
    }
    pub async fn put_message(&self, mut message: SecretMessage) -> DBResult<()> {
        if message.owner.is_empty() {
            return Err(anyhow!("owner must not be empty"));
        }
        if message.max_failed_verification < 1 || message.max_failed_verification > 9 {
            return Err(anyhow!("maximum consecutive failure should be between 1 and 9"));
        }
        if message.verify_every_minutes < 1 || message.verify_every_minutes > 4336204 {
            return Err(anyhow!(
                "maximum time between verification should be between 1 minute and 99 months"
            ));
        }
        if let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) {
            message.created_ts = now.as_secs();
        } else {
            return Err(anyhow!("cannot set message creation timestamp"));
        }
        let id = &Uuid::new_v4().to_string();
        message.id = id.to_string();

        self.db
            .fluent()
            .insert()
            .into(&self.message_coll)
            .document_id(id)
            .object(&message)
            .execute()
            .await?;

        info!("message upserted, Id: {}", id);
        Ok(())
    }
    pub async fn update_message_notified_on(&self, id: &str, email: &str) -> DBResult<()> {
        let mut message: SecretMessage = self.get_message(id).await?;
        if let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) {
            if email == message.recipient {
                message.recipient_notified_on = now.as_secs();
            } else if email == message.owner {
                message.owner_notified_on = now.as_secs();
            }
            self.db
                .fluent()
                .update()
                .in_col(&self.message_coll)
                .document_id(id)
                .object(&message)
                .execute()
                .await?;
        }
        Ok(())
    }

    pub async fn set_message_revealed_if_needed(&self, id: &str) -> DBResult<bool> {
        let mut m = self.get_message(id).await?;
        if m.revealed {
            return Ok(true);
        }
        // not revealed yet in db
        let owner = self.get_user(&m.owner).await?;
        if let Ok(r) = m.should_reveal(owner.last_seen) {
            if r {
                m.revealed = true;
                self.db
                    .fluent()
                    .update()
                    .in_col(&self.message_coll)
                    .document_id(id)
                    .object(&m)
                    .execute()
                    .await?;
                return Ok(true);
            }
        }
        Ok(false)
    }
    async fn get_message(&self, id: &str) -> DBResult<SecretMessage> {
        let m = self.db.fluent().select().by_id_in(&self.message_coll).obj().one(id).await?;
        m.ok_or_else(|| format_err!("cannot find message"))
    }
    pub async fn get_messages_for_email(
        &self,
        email: String,
    ) -> DBResult<Vec<MessageWithLastSeen>> {
        let messages: BTreeMap<String, SecretMessage> = self
            .get_all_messages()
            .await?
            .into_iter()
            .filter(|x| (x.1.owner == email || x.1.recipient == email))
            .collect();

        let mut out: Vec<MessageWithLastSeen> = Vec::new();
        for (k, v) in messages {
            let owner = self.get_user(&v.owner).await?;
            let recipient = self.get_user(&v.recipient).await?;
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

            // first set revealed on db if needed, this flag should only change from false
            // -> true once
            m.revealed = self.set_message_revealed_if_needed(&k).await?;

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

    pub async fn delete_message_from_email(
        &self,
        email: String,
        message_id: String,
    ) -> DBResult<()> {
        let messages: BTreeMap<String, SecretMessage> = self
            .get_all_messages()
            .await?
            .into_iter()
            .filter(|x| (x.1.owner == email || x.1.recipient == email))
            .collect();

        if messages.contains_key(message_id.as_str()) {
            let message = self.get_message(&message_id).await?;
            let should_delete = if email == message.recipient {
                self.set_message_revealed_if_needed(message_id.as_str()).await?
            } else {
                email == message.owner
            };
            if should_delete {
                self.db
                    .fluent()
                    .delete()
                    .from(&self.message_coll)
                    .document_id(&message_id)
                    .execute()
                    .await?;
                return Ok(());
            }
        }
        Err(anyhow!("message not found"))
    }

    pub async fn get_all_messages(&self) -> DBResult<BTreeMap<String, SecretMessage>> {
        let coll: Vec<SecretMessage> =
            self.db.fluent().select().from(self.message_coll.as_str()).obj().query().await?;

        let mut res: BTreeMap<String, SecretMessage> = BTreeMap::new();
        for sm in coll {
            res.insert(sm.id.clone(), sm);
        }
        Ok(res)
    }
    pub async fn unsubscribe_user(&self, email: String) -> DBResult<()> {
        let user: User = self.get_user(&email).await?;
        let new_user =
            User { id: user.id.clone(), last_seen: user.last_seen, ..Default::default() };
        self.put_user(new_user).await?;
        Ok(())
    }
    pub async fn subscribe_user(&self, email: String, sub: Subscription) -> DBResult<()> {
        let user: User = self.get_user(&email).await?;
        let new_user = User { id: user.id.clone(), last_seen: user.last_seen, subscription: sub };
        self.put_user(new_user).await?;
        Ok(())
    }
}
