use crate::{
    models::{url_type::UrlType, volume::GuildVolume},
    utils::{misc::ToEmoji, template::Template},
};
use serenity::{
    model::prelude::{ChannelId, UserId},
    prelude::Mentionable,
};
use std::{collections::HashMap, fmt::Display, fs, sync::OnceLock};
use toml::Table;

pub enum TurtoMessage<'a> {
    NotPlaying,
    UserNotInVoiceChannel,
    BotNotInVoiceChannel,
    DifferentVoiceChannel { bot: &'a ChannelId },
    Play { title: &'a str },
    Pause { title: &'a str },
    Stop { title: &'a str },
    Join(&'a ChannelId),
    Leave(&'a ChannelId),
    Queue { title: &'a str },
    Remove { title: &'a str },
    InvalidUrl(Option<&'a UrlType>),
    SetVolume(Result<GuildVolume, ()>),
    SetAutoleave(Result<bool, ()>),
    InvalidSeek { seek_limit: u64 },
    SeekNotAllow,
    BackwardSeekNotAllow,
    AdministratorOnly,
    UserGotBanned(Result<UserId, UserId>),
    UserGotUnbanned(Result<UserId, UserId>),
    InvalidUser,
    BannedUserResponse,
}

impl Display for TurtoMessage<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotPlaying => f.write_str(
                &TurtoMessageTemplates::get_template("not_playing")
                    .get_renderer()
                    .render_string(),
            ),
            Self::UserNotInVoiceChannel => f.write_str(
                &TurtoMessageTemplates::get_template("user_not_in_voice_channel")
                    .get_renderer()
                    .render_string(),
            ),
            Self::BotNotInVoiceChannel => f.write_str(
                &TurtoMessageTemplates::get_template("bot_not_in_voice_channel")
                    .get_renderer()
                    .render_string(),
            ),
            Self::DifferentVoiceChannel { bot } => f.write_str(
                &TurtoMessageTemplates::get_template("different_voice_channel")
                    .get_renderer()
                    .add_arg("bot_voice_channel", &bot.mention())
                    .render_string(),
            ),
            Self::Play { title } => f.write_str(
                &TurtoMessageTemplates::get_template("play")
                    .get_renderer()
                    .add_arg("title", title)
                    .render_string(),
            ),
            Self::Pause { title } => f.write_str(
                &TurtoMessageTemplates::get_template("pause")
                    .get_renderer()
                    .add_arg("title", title)
                    .render_string(),
            ),
            Self::Stop { title } => f.write_str(
                &TurtoMessageTemplates::get_template("stop")
                    .get_renderer()
                    .add_arg("title", title)
                    .render_string(),
            ),
            Self::Join(channel) => f.write_str(
                &TurtoMessageTemplates::get_template("join")
                    .get_renderer()
                    .add_arg("voice_channel", &channel.mention())
                    .render_string(),
            ),
            Self::Leave(channel) => f.write_str(
                &TurtoMessageTemplates::get_template("leave")
                    .get_renderer()
                    .add_arg("voice_channel", &channel.mention())
                    .render_string(),
            ),
            Self::Queue { title } => f.write_str(
                &TurtoMessageTemplates::get_template("queue")
                    .get_renderer()
                    .add_arg("title", title)
                    .render_string(),
            ),
            Self::Remove { title } => f.write_str(
                &TurtoMessageTemplates::get_template("remove")
                    .get_renderer()
                    .add_arg("title", title)
                    .render_string(),
            ),
            Self::InvalidUrl(url) => match url {
                Some(url_) => f.write_str(
                    &TurtoMessageTemplates::get_template("url_not_found")
                        .get_renderer()
                        .add_arg("url", url_)
                        .render_string(),
                ),
                None => f.write_str(
                    &TurtoMessageTemplates::get_template("invalid_url")
                        .get_renderer()
                        .render_string(),
                ),
            },
            Self::SetVolume(res) => match res {
                Ok(vol) => f.write_str(
                    &TurtoMessageTemplates::get_template("volume")
                        .get_renderer()
                        .add_arg("volume", &vol.to_emoji())
                        .render_string(),
                ),
                Err(_) => f.write_str(
                    &TurtoMessageTemplates::get_template("invalid_volume")
                        .get_renderer()
                        .render_string(),
                ),
            },
            Self::SetAutoleave(res) => match res {
                Ok(toggle) => match toggle {
                    true => f.write_str(
                        &TurtoMessageTemplates::get_template("toggle_autoleave")
                            .get_renderer()
                            .add_arg("autoleave_status", &"✅")
                            .render_string(),
                    ),
                    false => f.write_str(
                        &TurtoMessageTemplates::get_template("toggle_autoleave")
                            .get_renderer()
                            .add_arg("autoleave_status", &"❎")
                            .render_string(),
                    ),
                },
                Err(_) => f.write_str(
                    &TurtoMessageTemplates::get_template("invalid_autoleave")
                        .get_renderer()
                        .render_string(),
                ),
            },
            Self::InvalidSeek { seek_limit } => f.write_str(
                &TurtoMessageTemplates::get_template("invalid_seek")
                    .get_renderer()
                    .add_arg("seek_limit", seek_limit)
                    .render_string(),
            ),
            Self::SeekNotAllow => f.write_str(
                &TurtoMessageTemplates::get_template("seek_not_allow")
                    .get_renderer()
                    .render_string(),
            ),
            Self::BackwardSeekNotAllow => f.write_str(
                &TurtoMessageTemplates::get_template("backward_seek_not_allow")
                    .get_renderer()
                    .render_string(),
            ),
            Self::AdministratorOnly => f.write_str(
                &TurtoMessageTemplates::get_template("administrator_only")
                    .get_renderer()
                    .render_string(),
            ),
            Self::UserGotBanned(user) => match user {
                Ok(u) => f.write_str(
                    &TurtoMessageTemplates::get_template("user_got_banned")
                        .get_renderer()
                        .add_arg("user", &u.mention())
                        .render_string(),
                ),
                Err(u) => f.write_str(
                    &TurtoMessageTemplates::get_template("user_already_banned")
                        .get_renderer()
                        .add_arg("user", &u.mention())
                        .render_string(),
                ),
            },
            Self::UserGotUnbanned(user) => match user {
                Ok(u) => f.write_str(
                    &TurtoMessageTemplates::get_template("user_got_unbanned")
                        .get_renderer()
                        .add_arg("user", &u.mention())
                        .render_string(),
                ),
                Err(u) => f.write_str(
                    &TurtoMessageTemplates::get_template("user_not_banned")
                        .get_renderer()
                        .add_arg("user", &u.mention())
                        .render_string(),
                ),
            },
            Self::InvalidUser => f.write_str(
                &TurtoMessageTemplates::get_template("invalid_user")
                    .get_renderer()
                    .render_string(),
            ),
            Self::BannedUserResponse => f.write_str(
                &TurtoMessageTemplates::get_template("banned_user_repsonse")
                    .get_renderer()
                    .render_string(),
            ),
        }
    }
}

struct TurtoMessageTemplates;

impl TurtoMessageTemplates {
    fn get_template(name: &str) -> &Template {
        static TEMPLATES: OnceLock<HashMap<String, Template>> = OnceLock::new();
        TEMPLATES
            .get_or_init(|| {
                let templates_map = fs::read_to_string("templates.toml")
                    .map_err(|err| panic!("Error loading templates.toml: {err}"))
                    .and_then(|templates_toml| toml::from_str::<Table>(&templates_toml))
                    .unwrap_or_else(|err| panic!("Error parsing templates.toml: {err}"));

                templates_map
                    .into_iter()
                    .map(|(k, v)| {
                        let template_str = v.as_str().unwrap_or_else(|| {
                            panic!("The value of template \"{k}\" should be a string")
                        });
                        let template = Template::parse(template_str)
                            .map_err(|err| panic!("Error parsing template \"{k}\": {err}"))
                            .unwrap();
                        println!("k: {} template: {:?}", k, template);
                        (k, template)
                    })
                    .collect::<HashMap<_, _>>()
            })
            .get(name)
            .unwrap_or_else(|| {
                panic!(
                    "Can't find the message template \"{}\" in templates.toml",
                    name
                )
            })
    }
}
