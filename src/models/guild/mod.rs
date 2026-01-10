use crate::utils::json::{read_json, write_json};
use dashmap::DashMap;
use serenity::all::GuildId;
use std::{
    ops::{Deref, DerefMut},
    path::Path,
};

pub mod config;

pub mod data;
use data::GuildData;

pub mod volume;

#[derive(Debug, Default)]
pub struct Guilds(DashMap<GuildId, GuildData>);

impl Deref for Guilds {
    type Target = DashMap<GuildId, GuildData>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Guilds {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Guilds {
    pub fn load(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let map = read_json(path)?;
        Ok(Self(map))
    }

    pub fn save(&self, path: impl AsRef<Path>) -> std::io::Result<usize> {
        write_json(&self.0, path)
    }
}
