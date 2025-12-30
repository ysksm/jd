use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
}

impl Project {
    pub fn new(id: String, key: String, name: String, description: Option<String>) -> Self {
        Self {
            id,
            key,
            name,
            description,
        }
    }
}
