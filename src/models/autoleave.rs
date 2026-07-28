use poise::ChoiceParameter;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, ChoiceParameter, Serialize, Deserialize, PartialEq, Clone, Copy, strum::Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum AutoleaveType {
    #[name = "on"]
    On,
    #[name = "empty"]
    Empty,
    #[name = "silent"]
    Silent,
    #[name = "off"]
    Off,
}

impl AutoleaveType {
    /// Leave once the queue has drained. The policy for the end of the last track,
    /// shared by the autonomous advance and the `skip` command so the two cannot
    /// diverge.
    pub fn leaves_on_empty_queue(self) -> bool {
        matches!(self, Self::On | Self::Silent)
    }

    /// Leave once the last other user has left the bot's voice channel.
    pub fn leaves_on_empty_channel(self) -> bool {
        matches!(self, Self::On | Self::Empty)
    }
}
