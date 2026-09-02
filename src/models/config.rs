use paste::paste;
use serde::{Deserialize, Serialize};
use serenity::model::prelude::UserId;
use std::{path::Path, sync::Arc};
use tracing::warn;

/// Define config structs
///
/// # What does this do?
/// - Define config struct
/// - Define function returning default value for each field
/// - Implement [`Default`] for it
/// - Let serde to use the default function
/// - Forward outer attributes (doc comments included) on the struct and on each field
macro_rules! define_config {
    ($(#[$meta:meta])* $vis:vis struct $name:ident {
        $($(#[$field_meta:meta])* $v:vis $field:ident: $typ:ty = $def:expr),* $(,)?
    }) => {
        paste! {
            $(#[$meta])*
            #[derive(Debug, Serialize, Deserialize)]
            $vis struct $name {
                $(
                    $(#[$field_meta])*
                    #[serde(default = $name "::default_" $field)]
                    $v $field: $typ,
                )*
            }

            impl Default for $name {
                fn default() -> Self {
                    Self {
                        $($field: Self::[<default_ $field>](),)*
                    }
                }
            }

            impl $name {
                $(
                    #[inline]
                    fn [<default_ $field>]() -> $typ {
                        $def
                    }
                )*
            }
        }

    };
}

define_config! {
    /// The config of the bot, loaded from `config.toml`
    pub struct TurtoConfig {
        /// Whether seeking is allowed, seeking can be "expensive" and cause the bot lagging
        pub allow_seek: bool = true,
        /// Whether backward seeking is allowed, it is usually more "expensive" than forward seeking
        pub allow_backward_seek: bool = false,
        /// The duration limitation of seeking, denoted by seconds
        pub seek_limit: u64 = 600,
        /// Only one command can be invoked in this amount of seconds, counted per guild
        pub command_delay: u64 = 1,
        /// The owner of this bot, denoted by a Discord user id
        pub owner: Option<UserId> = None,
        /// Whether the data is saved at intervals, otherwise it is only saved on shutdown
        pub auto_save: bool = true,
        /// The interval of auto saving, denoted by seconds
        pub auto_save_interval: u64 = 3600,
        /// The config of the yt-dlp sidecar
        pub ytdlp: Arc<YtdlpConfig> = Arc::new(YtdlpConfig::default()),
    }
}

define_config! {
    /// The config of the yt-dlp sidecar, corresponding to the `[ytdlp]` table of `config.toml`
    pub struct YtdlpConfig {
        /// Use nightly builds of yt-dlp
        pub use_nightly: bool = false,
        /// Use the bun that is already in the environment as the yt-dlp JS runtime
        pub use_system_bun: bool = false,
        /// Use the uv that is already in the environment
        pub use_system_uv: bool = false,
        /// The path to the YouTube cookies in Netscape format
        pub cookies_path: Option<String> = None,
        /// How many extractions and downloads the yt-dlp sidecar may run at once, the rest queue up
        pub max_concurrency: u32 = 8,
    }
}

impl TurtoConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, toml::de::Error> {
        let path = path.as_ref();
        let config_str = match std::fs::read_to_string(path) {
            Ok(file) => file,
            Err(error) => {
                tracing::warn!(%error, path = %path.display(), "failed to open config file, will use default");
                return Ok(Self::default());
            }
        };

        let de = toml::Deserializer::new(&config_str);
        let config: TurtoConfig = serde_ignored::deserialize(de, |attribute| {
            tracing::warn!(
                %attribute,
                "unknown config attribute ignored"
            )
        })?;

        if config.owner.is_none() {
            warn!("The owner of this bot hasn't been set");
        }
        Ok(config)
    }

    pub fn is_owner(&self, user: &UserId) -> bool {
        if let Some(owner) = &self.owner {
            return owner == user;
        }
        false
    }
}
