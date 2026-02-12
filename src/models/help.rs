use crate::{commands::CommandKind, for_each_cmd};
use paste::paste;
use regex::Regex;
use serenity::all::CreateEmbed;
use std::{borrow::Cow, collections::HashMap, path::Path, sync::LazyLock};

#[derive(Debug, serde::Deserialize)]
pub struct CommandHelp<C>
where
    C: Command,
    C::Params: for<'d> serde::Deserialize<'d>,
    CommandHelp<C>: Default,
{
    pub short_description: Option<String>,
    pub description: Option<String>,
    pub parameters: Option<C::Params>,
}

impl<C> CommandHelp<C>
where
    C: Command,
    C::Params: for<'d> serde::Deserialize<'d>,
    CommandHelp<C>: Default,
{
    fn command_name(&self) -> &'static str {
        C::NAME
    }

    fn view_default() -> CommandHelpView<'static> {
        CommandHelpView {
            name: Cow::Borrowed(C::NAME),
            short_description: Cow::Borrowed(C::DEFAULT_SHORT_DESCRIPTION),
            description: Cow::Borrowed(C::DEFAULT_DESCRIPTION),
            parameters: C::Params::view_default(),
        }
    }

    fn view_with_fallback<'a>(&'a self) -> CommandHelpView<'a> {
        let parameters = match self.parameters {
            Some(ref params) => params.view_with_fallback(),
            None => C::Params::view_default(),
        };
        CommandHelpView {
            name: Cow::Borrowed(C::NAME),
            short_description: Cow::Borrowed(
                self.short_description
                    .as_deref()
                    .unwrap_or(C::DEFAULT_SHORT_DESCRIPTION),
            ),
            description: Cow::Borrowed(
                self.description
                    .as_deref()
                    .unwrap_or(C::DEFAULT_DESCRIPTION),
            ),
            parameters,
        }
    }

    fn view_locale<'a>(&'a self) -> LocaleCommandHelpView<'a> {
        LocaleCommandHelpView {
            short_description: self.short_description.as_deref().map(Cow::Borrowed),
            description: self.description.as_deref().map(Cow::Borrowed),
            params: self.parameters.as_ref().map(Params::view_locale),
        }
    }
}

pub trait Command {
    type Params: Params;
    const NAME: &str;
    const DEFAULT_DESCRIPTION: &str;
    const DEFAULT_SHORT_DESCRIPTION: &str;
}

pub trait Params {
    fn view_default() -> HashMap<Cow<'static, str>, Cow<'static, str>>;
    fn view_with_fallback<'a>(&'a self) -> HashMap<Cow<'a, str>, Cow<'a, str>>;
    fn view_locale<'a>(&'a self) -> HashMap<Cow<'a, str>, Cow<'a, str>>;
}

/// Define the command help and parameters structs
///
/// # What does this do?
///
/// - Define the command struct used for [`CommandHelp`] generic
/// - Define the default [`CommandHelp::description`] and [`CommandHelp::short_description`] for this command
/// - Implement [`Command`] for the command struct
/// - Define the struct for parameters of the command and used for [`Command::Params`]
/// - Define default description for all parameters
/// - Implement [`Params`] for the parameters struct
/// - Implement [`Default`] for them
///
/// # Example
/// ```rust
/// defind_cmd! {
///     Cmd : CmdParams {
///         short_description: "some short description",
///         description: "some description",
///         parameters: {
///             param: "parameter description"
///         }
///     }
/// }
/// ```
macro_rules! define_cmd {
    (
        $name:ident : $params:ident {
            short_description: $short_desc:literal,
            description: $desc:literal,
            parameters: {
                $($field:ident : $def:literal),* $(,)?
            }
        }
    ) => {
        paste! {
            #[derive(Debug)]
            pub struct $name;

            #[derive(Debug, serde::Deserialize)]
            pub struct $params {
                $(
                    $field: Option<String>
                ),*
            }

            impl Command for $name {
                type Params = $params;
                const NAME: &str = stringify!([<$name:lower>]);
                const DEFAULT_DESCRIPTION: &str = $desc;
                const DEFAULT_SHORT_DESCRIPTION: &str = $short_desc;
            }

            impl Default for CommandHelp<$name> {
                fn default() -> CommandHelp<$name> {
                    CommandHelp {
                        short_description: Some($name::DEFAULT_SHORT_DESCRIPTION.to_owned()),
                        description: Some($name::DEFAULT_DESCRIPTION.to_owned()),
                        parameters: Some($params::default())
                    }
                }
            }

            impl Default for $params {
                fn default() -> Self {
                    Self {
                        $($field: Some($def.to_owned())),*
                    }
                }
            }

            impl Params for $params {
                #[allow(unused_mut)]
                fn view_default() -> HashMap<Cow<'static, str>, Cow<'static, str>> {
                    let mut map = HashMap::new();
                    $(
                        map.insert(
                            Cow::Borrowed(stringify!($field)),
                            Cow::Borrowed(Self::[<DEFAULT_ $field:upper>])
                        );
                    )*
                    map
                }

                #[allow(unused_mut)]
                fn view_with_fallback<'a>(&'a self) -> HashMap<Cow<'a, str>, Cow<'a, str>> {
                    let mut map = HashMap::new();
                    $(
                        if let Some(desc) = self.$field.as_deref() {
                            map.insert(
                                Cow::Borrowed(stringify!($field)),
                                Cow::Borrowed(desc)
                            );
                        } else {
                            map.insert(
                                Cow::Borrowed(stringify!($field)),
                                Cow::Borrowed(Self::[<DEFAULT_ $field:upper>])
                            );
                        }
                    )*
                    map
                }

                #[allow(unused_mut)]
                fn view_locale<'a>(&'a self) -> HashMap<Cow<'a, str>, Cow<'a, str>> {
                    let mut map = HashMap::new();
                    $(
                        if let Some(desc) = self.$field.as_deref() {
                            map.insert(
                                Cow::Borrowed(stringify!($field)),
                                Cow::Borrowed(desc)
                            );
                        }
                    )*
                    map
                }
            }

            impl $params {
                $(
                    const [<DEFAULT_ $field:upper>]: &str = $def;
                )*
            }
        }
    }
}

define_cmd! {
    Autoleave : AutoleaveParams {
        short_description: "Toggle automatic leaving.",
        description: "Enable (`on`, `empty`, `silent`) or disable (`off`) automatic leaving. When automatic leaving is enabled, turto will leave the voice channel automatically when the playlist is empty after playback ends or is stopped.\n\
                    - `on`: turto will leave when nothing is playing or no one is in the voice channel\n\
                    - `empty`: turto will leave when no one is in the voice channel\n\
                    - `silent`: turto will leave when no nothing is playing\n\
                    - `off`: turto won't leave automatically",
        parameters: {
            toggle: "Toggle autoleave, refer to help command for usage",
        }
    }
}

define_cmd! {
    Join : JoinParams {
        short_description: "Let turto join the voice channel you are in.",
        description: "Let turto join the voice channel you are in. It has no effect if turto is already in another voice channel.",
        parameters: {}
    }
}

define_cmd! {
    Leave : LeaveParams {
        short_description: "Let turto leave the current voice channel.",
        description: "Let turto leave the current voice channel.",
        parameters: {}
    }
}

define_cmd! {
    Pause : PauseParams {
        short_description: "Pause the currently playing item.",
        description: "Pause the currently playing item.",
        parameters: {}
    }
}

define_cmd! {
    Play : PlayParams {
        short_description: "Start playback.",
        description: "Start playback. If turto is not in another voice channel, it will join your current one. Depending on the situation, there are several possibilities:\n\
                    1. If `url` is provided, it will interrupt the currently playing item, and start playing it. Supported sources include YouTube, Bilibili videos and Soundcloud music (you can try other platform, as long as it's supported by yt-dlp).\n\
                    2. If no `url` is provided and there is a paused item, it will resume playing that item.\n\
                    3. If no `url` is provided and there is no paused item, it will start playing the playlist from the beginning.",
        parameters: {
            url: "Optional, the link to what you want to play"
        }
    }
}

define_cmd! {
    Playlist : PlaylistParams {
        short_description: "Display the playlist.",
        description: "Display the current playlist, which is shared across the entire server. You can specify `page` for the page number to directly display certain page, or use the select menu.",
        parameters: {
            page: "Optional, the page to display"
        }
    }
}

define_cmd! {
    Playwhat : PlaywhatParams {
        short_description: "Display the currently playing item.",
        description: "Display the currently playing item.",
        parameters: {}
    }
}

define_cmd! {
    Queue : QueueParams {
        short_description: "Add new item to the end of playlist.",
        description: "Add new item to the end of playlist, the parameter `url` can be any URL. Supported various platforms, as long as it's supported by yt-dlp. You can also directly add entire YouTube playlists, and playlist URLs will be prioritized.",
        parameters: {
            url: "The link to what you want to enqueue"
        }
    }
}

define_cmd! {
    Seek : SeekParams {
        short_description: "Seek the currently playing item to certain time",
        description: "If there is a currently playing or paused item, jump to the specified `time` in seconds.",
        parameters: {
            time: "The time to seek, denoted in second"
        }
    }
}

define_cmd! {
    Skip : SkipParams {
        short_description: "Skip the currently playing item.",
        description: "Skip the currently playing item, and start playing the next item in the playlist.",
        parameters: {}
    }
}

define_cmd! {
    Stop : StopParams {
        short_description: "Stop the currently playing item.",
        description: "Stop the currently playing item.",
        parameters: {}
    }
}

define_cmd! {
    Volume : VolumeParams {
        short_description: "Adjust the volume",
        description: "Adjust the volume to `value`, which can range from 0 (mute) to 100. The volume setting is shared across the entire server.",
        parameters: {
            value: "The value of volume, range from 0 to 100"
        }
    }
}

define_cmd! {
    Ban : BanParams {
        short_description: "Ban a user",
        description: "Ban a user, then the banned user won't be able to use any command.",
        parameters: {
            user: "The user to be banned"
        }
    }
}

define_cmd! {
    Unban : UnbanParams {
        short_description: "Unban a user",
        description: "Unban a user (if banned), then the user will be able to use all commands.",
        parameters: {
            user: "The user to be unbanned"
        }
    }
}

define_cmd! {
    Shuffle : ShuffleParams {
        short_description: "Shuffle the playlist.",
        description: "Shuffle the playlist.",
        parameters: {}
    }
}

define_cmd! {
    Repeat : RepeatParams {
        short_description: "Toggle repeating",
        description: "Enable (`on`) or disable (`off`) repeating. When repeating is enabled, turto will repeatly playing the currently playing item.",
        parameters: {
            toggle: "Can be`on` or `off`, to toggle repeat function"
        }
    }
}

define_cmd! {
    About : AboutParams {
        short_description: "Display the information about this bot.",
        description: "Display the information about this bot.",
        parameters: {}
    }
}

define_cmd! {
    Remove : RemoveParams {
        short_description: "Delete items from the playlist.",
        description: "Delete certain items from the playlist, there are two ways to use it:\n\
                    1. You can delete the item at position `which` in the playlist, by specifying the `which` parameter.\n\
                    2. You can delete all items between positions `which` and `to_which` in the playlist, by specifying both `which` and `to_which` parameters.",
        parameters: {
            which: "Which item to remove",
            to_which: "Optional, if specified, remove all items within the range `which` to `to_which`"
        }
    }
}

define_cmd! {
    Help : HelpParams {
        short_description: "Look up how to use each command",
        description: "Look up how to use each command",
        parameters: {
            command: "The command to look up"
        }
    }
}

define_cmd! {
    Insert : InsertParams {
        short_description: "Add new item to the beginning of playlist.",
        description: "Add new item to the beginning of playlist, the parameter `url` can be any URL. Supported various platforms, as long as it's supported by yt-dlp. You can also directly add entire YouTube playlists, and playlist URLs will be prioritized.",
        parameters: {
            url: "The link to what you want to insert"
        }
    }
}

define_cmd! {
    Clear : ClearParams {
        short_description: "Clear the playlist.",
        description: "Clear the playlist.",
        parameters: {}
    }
}

/// Define the [`HelpLocale`]  and use all commands for its fields
///
/// Should be use with [`for_each_cmd`] macro
macro_rules! define_help_locale {
    ($($cmd:ident),* $(,)?) => {
        paste! {
            #[derive(Debug, Default, serde::Deserialize)]
            pub struct HelpLocale {
                $(
                    pub $cmd: Option<CommandHelp<[<$cmd:camel>]>>,
                )*
            }
        }
    };
}

for_each_cmd!(define_help_locale);

impl HelpLocale {
    pub fn view<'a>(&'a self) -> HashMap<&'static str, LocaleCommandHelpView<'a>> {
        let mut map = HashMap::new();

        /// Generate code to create [`LocaleCommandHelpView`] for each command to the map if present
        ///
        /// Will generate code like below for each command,
        /// should be used with [`for_each_cmd`] macro
        ///
        /// ```
        /// if let Some(ref about) = self.about {
        ///     let cmd_view = cmd.view_locale();
        ///     map.insert(cmd.command_name(), cmd_view);
        /// }
        /// ```
        macro_rules! add_view {
            ($($cmd:ident),* $(,)?) => {
                $(
                    if let Some(ref cmd) = self.$cmd {
                        let cmd_view = cmd.view_locale();
                        map.insert(cmd.command_name(), cmd_view);
                    }
                )*
            };
        }

        for_each_cmd!(add_view);

        map
    }
}

pub struct LocaleCommandHelpView<'a> {
    pub short_description: Option<Cow<'a, str>>,
    pub description: Option<Cow<'a, str>>,
    pub params: Option<HashMap<Cow<'a, str>, Cow<'a, str>>>,
}

pub struct CommandHelpView<'a> {
    pub name: Cow<'a, str>,
    pub short_description: Cow<'a, str>,
    pub description: Cow<'a, str>,
    pub parameters: HashMap<Cow<'a, str>, Cow<'a, str>>,
}

impl<'a> CommandHelpView<'a> {
    pub fn create_embed(&self) -> CreateEmbed {
        let mut embed = CreateEmbed::new()
            .title(self.name.as_ref())
            .description(self.description.as_ref());

        for (param, desc) in self.parameters.iter() {
            embed = embed.field(param.as_ref(), desc.as_ref(), false);
        }

        embed
    }
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct HelpConfig {
    #[serde(default)]
    default: HelpLocale,

    #[serde(flatten, default)]
    locales: HashMap<String, HelpLocale>,
}

impl HelpConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, toml::de::Error> {
        fn log_unknown(path: &serde_ignored::Path<'_>) {
            static REGEX: LazyLock<Regex> = LazyLock::new(|| {
                Regex::new(
                    r"(?<locale>\w+)\.(?<command>\w+)(\.\?\.(?<attr>\w+))?(\.\?\.(?<param>\w+))?",
                )
                .unwrap()
            });

            let path = path.to_string();
            if let Some(cap) = REGEX.captures(&path) {
                let locale = cap.name("locale").unwrap();
                let command = cap.name("command").unwrap();
                match cap.name("attr") {
                    Some(attr) => {
                        if attr.as_str() == "parameters"
                            && let Some(param) = cap.name("param")
                        {
                            tracing::warn!(
                                locale = locale.as_str(),
                                command = command.as_str(),
                                parameter = param.as_str(),
                                "unknown or invalid command parameter ignored"
                            );
                        } else {
                            tracing::warn!(
                                locale = locale.as_str(),
                                command = command.as_str(),
                                attribute = attr.as_str(),
                                "unknown command attribute ignored"
                            );
                        }
                    }
                    None => {
                        tracing::warn!(
                            locale = locale.as_str(),
                            command = command.as_str(),
                            "unknown command ignored"
                        );
                    }
                }
            }
        }

        let path = path.as_ref();
        let help_str = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(error) => {
                tracing::warn!(%error, path = %path.display(), "failed to load help messages, will use default");
                return Ok(HelpConfig::default());
            }
        };
        let de = toml::Deserializer::new(&help_str);
        let map: HelpConfig = serde_ignored::deserialize(de, |path| log_unknown(&path))?;

        Ok(map)
    }

    pub fn iter_locale(&self) -> impl Iterator<Item = (&str, &HelpLocale)> {
        self.locales
            .iter()
            .map(|(locale, help_locale)| (locale.as_str(), help_locale))
    }

    pub fn view_default_locale_command<'a>(&'a self, command: CommandKind) -> CommandHelpView<'a> {
        /// Generate match arms for each command to create [`CommandHelpView`]
        ///
        /// Will generate code like below for each command,
        /// should be used with [`for_each_cmd`] macro
        ///
        /// ```rust
        /// match command {
        ///     CommandKind::about => CommandHelp::<About>::view_default()
        /// }
        /// ```
        macro_rules! match_default {
            ($($cmd:ident),* $(,)?) => {
                paste! {
                    match command {
                        $(
                            CommandKind::$cmd => match self.default.$cmd {
                                Some(ref cmd_help) => cmd_help.view_with_fallback(),
                                None => CommandHelp::<[<$cmd:camel>]>::view_default()
                            },
                        )*
                    }
                }
            };
        }
        for_each_cmd!(match_default)
    }

    pub fn view_locale_command_with_fallback<'a>(
        &'a self,
        locale: Option<&str>,
        command: CommandKind,
    ) -> CommandHelpView<'a> {
        let Some(locale) = locale else {
            return self.view_default_locale_command(command);
        };
        let Some(help_locale) = self.locales.get(locale) else {
            return self.view_default_locale_command(command);
        };

        /// Generate match arm for each command to create [`CommandHelpView`],
        /// fallback to default when command help is not present in [`HelpLocale`]
        ///
        /// Will generate code like below for each command,
        /// should be used with [`for_each_cmd`] macro
        /// ```rust
        /// match command {
        ///     CommandKind::about => {
        ///         let Some(ref help) = help_locale.about else {
        ///             return self.view_default_locale_command(command);
        ///         };
        ///         return help.view_with_fallback();
        ///     }
        /// }
        /// ```
        macro_rules! match_help {
            ($($cmd:ident),* $(,)?) => {
                paste! {
                    match command {
                        $(
                            CommandKind::$cmd => {
                                let Some(ref help) = help_locale.$cmd else {
                                    return self.view_default_locale_command(command);
                                };
                                return help.view_with_fallback();
                            }
                        )*
                    }
                }
            };
        }

        for_each_cmd!(match_help)
    }
}
