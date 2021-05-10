use chrono::prelude::*;
use uuid::Uuid;

use crate::model::session::Session;

#[derive(Debug, Clone)]
pub struct Message {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub body: String,
    // TODO: the kanoogi-user object is just for quick-lookups... we don't want to store this in the DB< just the user UUID
    pub user: Session,
    // This is the Kanoogi-Session UUID
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

impl Message {
    pub fn new(
        id: Uuid,
        channel_id: Uuid,
        user: Session,
        body: &str,
        created_at: DateTime<Utc>,
    ) -> Self {
        Message {
            id,
            channel_id,
            created_by: user.id,
            user,
            body: String::from(body),
            created_at,
        }
    }
}
