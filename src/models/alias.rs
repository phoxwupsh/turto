use super::{data::Data, error::CommandError};

pub type Context<'a> = poise::Context<'a, Data, CommandError>;
pub type Command = poise::Command<Data, CommandError>;
