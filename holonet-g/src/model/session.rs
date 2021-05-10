use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: Uuid,
    pub name: String,
}

impl Session {
    pub fn new(id: Uuid, name: &str) -> Self {
        Session {
            id,
            name: String::from(name),
        }
    }
}
