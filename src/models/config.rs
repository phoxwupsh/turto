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
macro_rules! define_config {
    ($name:ident {
        $($v:vis $field:ident: $typ:ty = $def:expr),* $(,)?
    }) => {
        paste! {
            #[derive(Debug, Serialize, Deserialize)]
            pub struct $name {
                $(
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
    TurtoConfig {
        pub allow_seek: bool = true,
        pub allow_backward_seek: bool = false,
        pub seek_limit: u64 = 600,
        pub command_delay: u64 = 1,
        pub owner: Option<UserId> = None,
        pub auto_save: bool = true,
        pub auto_save_interval: u64 = 3600,
        pub ytdlp: Arc<YtdlpConfig> = Arc::new(YtdlpConfig::default()),
    }
}

define_config! {
    YtdlpConfig {
        pub use_system_ytdlp: bool = false,
        pub use_nightly: bool =false,
        pub use_system_bun: bool = false,
        pub cookies_path: Option<String> = None,
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
