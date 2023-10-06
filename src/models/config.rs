use serde::{Deserialize, Serialize};
use serenity::model::prelude::UserId;

#[derive(Debug, Serialize, Deserialize)]
pub struct TurtoConfig {
    pub command_prefix: String,
    pub allow_seek: bool,
    pub allow_backward_seek: bool,
    pub seek_limit: u64,
    pub command_delay: u64,
    pub owner: Option<UserId>
}

impl TurtoConfig {
    pub fn is_owner(&self, user: &UserId) -> bool {
        if let Some(owner) = &self.owner {
            return owner == user
        }
        false
    }
}