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
