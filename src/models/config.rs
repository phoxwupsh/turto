use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TurtoConfig {
    pub command_prefix: String,
    pub allow_seek: bool,
    pub allow_backward_seek: bool,
    pub seek_limit: u64,
    pub command_delay: u64,
}
