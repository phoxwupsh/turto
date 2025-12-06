use poise::ChoiceParameter;
use serde::{Deserialize, Serialize};

#[derive(Debug, ChoiceParameter, Serialize, Deserialize, PartialEq, Clone, Copy)]
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
