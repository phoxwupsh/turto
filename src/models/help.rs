use crate::models::command::CommandKind;
use serenity::all::CreateEmbed;
use std::{collections::BTreeMap, path::Path, str::FromStr};

/// Discord's hard limit on a slash command's description.
const SHORT_DESCRIPTION_LIMIT: usize = 100;

/// The locales Discord accepts as keys in a localization map. Neither serenity nor poise
/// exposes this list, and Discord rejects the whole command registration if it sees a key
/// outside it — so an unrecognized locale is dropped at load rather than at startup.
///
/// <https://discord.com/developers/docs/reference#locales>
const DISCORD_LOCALES: &[&str] = &[
    "id", "da", "de", "en-GB", "en-US", "es-ES", "es-419", "fr", "hr", "it", "lt", "hu", "nl",
    "no", "pl", "pt-BR", "ro", "fi", "sv-SE", "vi", "tr", "cs", "el", "bg", "ru", "uk", "hi", "th",
    "zh-CN", "ja", "zh-TW", "ko",
];

/// One command's help as read from a help file. Every field is optional: anything
/// omitted falls back to the [`CommandEntry`](crate::models::command::CommandEntry) default during [`HelpConfig::resolve`].
///
/// The same shape is reused for every command, so the whole help file deserializes
/// into plain maps — no per-command types are needed.
#[derive(Debug, Default, serde::Deserialize)]
struct RawCommandHelp {
    short_description: Option<String>,
    description: Option<String>,
    #[serde(default)]
    parameters: BTreeMap<String, String>,
}

/// The help file exactly as written on disk. Command names are plain strings here, so
/// an unknown or misspelled command never fails deserialization — it is dropped (with a
/// warning) when converted into [`HelpConfig`]. Mirrors the `[default.<cmd>]` /
/// `[<locale>.<cmd>]` TOML layout.
#[derive(Debug, Default, serde::Deserialize)]
struct RawHelpConfig {
    #[serde(default)]
    default: BTreeMap<String, RawCommandHelp>,

    #[serde(flatten, default)]
    locales: BTreeMap<String, BTreeMap<String, RawCommandHelp>>,
}

/// The validated, runtime help config: a default locale plus any number of named
/// locales, each a `command -> help` map keyed by the typed [`CommandKind`]. Built from
/// [`RawHelpConfig`] in [`HelpConfig::load`].
#[derive(Debug, Default)]
pub struct HelpConfig {
    default: BTreeMap<CommandKind, RawCommandHelp>,
    locales: BTreeMap<String, BTreeMap<CommandKind, RawCommandHelp>>,
}

/// A command's help resolved for one locale, ready to render. Borrows from both the
/// [`HelpConfig`] (overrides) and the `'static` [`CommandEntry`](crate::models::command::CommandEntry) (defaults).
pub struct ResolvedHelp<'a> {
    pub name: &'a str,
    pub short_description: &'a str,
    pub description: &'a str,
    pub parameters: BTreeMap<&'a str, &'a str>,
}

/// One locale's overrides for a command, exposed without leaking the raw help shape.
/// A field is `None` when that locale doesn't translate it.
pub struct LocaleOverride<'a> {
    pub locale: &'a str,
    raw: &'a RawCommandHelp,
}

impl<'a> LocaleOverride<'a> {
    pub fn short_description(&self) -> Option<&'a str> {
        self.raw.short_description.as_deref()
    }

    pub fn parameter(&self, name: &str) -> Option<&'a str> {
        self.raw.parameters.get(name).map(String::as_str)
    }
}

impl ResolvedHelp<'_> {
    pub fn create_embed(&self) -> CreateEmbed {
        let mut embed = CreateEmbed::new()
            .title(self.name)
            .description(self.description);

        for (param, desc) in self.parameters.iter() {
            embed = embed.field(*param, *desc, false);
        }

        embed
    }
}

impl HelpConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, toml::de::Error> {
        let path = path.as_ref();
        let help_str = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(error) => {
                tracing::warn!(%error, path = %path.display(), "failed to load help messages, will use default");
                return Ok(HelpConfig::default());
            }
        };
        let de = toml::Deserializer::new(&help_str);
        let raw: RawHelpConfig = serde_ignored::deserialize(de, |path| {
            tracing::warn!(field = %path, "unknown help field ignored");
        })?;

        Ok(HelpConfig::from_raw(raw))
    }

    /// Convert the on-disk shape into the typed config, resolving each command name to a
    /// [`CommandKind`] and logging — then dropping — anything that isn't a real command.
    ///
    /// Every top-level key other than `default` is a locale. Because the key is free-form,
    /// a typo like `[defualt.play]` would otherwise become a silently-inert locale, so
    /// unknown locales are dropped here too.
    fn from_raw(raw: RawHelpConfig) -> Self {
        HelpConfig {
            default: index_commands("default", raw.default),
            locales: raw
                .locales
                .into_iter()
                .filter_map(|(locale, cmds)| {
                    if !DISCORD_LOCALES.contains(&locale.as_str()) {
                        tracing::warn!(
                            locale,
                            "unknown locale ignored, expected `default` or a locale Discord \
                             supports"
                        );
                        return None;
                    }
                    let indexed = index_commands(&locale, cmds);
                    Some((locale, indexed))
                })
                .collect(),
        }
    }

    /// The locale overrides for `command`, one per locale that actually customizes it.
    /// Used to register Discord's per-locale slash command descriptions.
    pub fn locale_overrides(
        &self,
        command: CommandKind,
    ) -> impl Iterator<Item = LocaleOverride<'_>> {
        self.locales.iter().filter_map(move |(locale, cmds)| {
            cmds.get(&command).map(|raw| LocaleOverride {
                locale: locale.as_str(),
                raw,
            })
        })
    }

    /// Resolve a command's help for `locale`, falling back per field:
    /// locale override -> default-locale override -> registered [`CommandEntry`](crate::models::command::CommandEntry) default.
    /// Passing `locale = None` resolves the default locale.
    pub fn resolve(&self, locale: Option<&str>, command: CommandKind) -> ResolvedHelp<'_> {
        let entry = command.entry();

        let locale_raw = locale
            .and_then(|locale| self.locales.get(locale))
            .and_then(|cmds| cmds.get(&command));
        let default_raw = self.default.get(&command);

        let short_description = locale_raw
            .and_then(|raw| raw.short_description.as_deref())
            .or_else(|| default_raw.and_then(|raw| raw.short_description.as_deref()))
            .unwrap_or(entry.short_description);

        let description = locale_raw
            .and_then(|raw| raw.description.as_deref())
            .or_else(|| default_raw.and_then(|raw| raw.description.as_deref()))
            .unwrap_or(entry.description);

        let parameters = entry
            .parameters
            .iter()
            .map(|param| {
                let desc = locale_raw
                    .and_then(|raw| raw.parameters.get(param.name).map(String::as_str))
                    .or_else(|| {
                        default_raw
                            .and_then(|raw| raw.parameters.get(param.name).map(String::as_str))
                    })
                    .unwrap_or(param.description);
                (param.name, desc)
            })
            .collect();

        ResolvedHelp {
            name: entry.name,
            short_description,
            description,
            parameters,
        }
    }
}

/// Index a locale's commands by [`CommandKind`], dropping (with a warning) any name that
/// isn't a real command and warning about parameter names that don't exist on the
/// command. Validated against the command registry.
fn index_commands(
    locale: &str,
    cmds: BTreeMap<String, RawCommandHelp>,
) -> BTreeMap<CommandKind, RawCommandHelp> {
    cmds.into_iter()
        .filter_map(|(command, mut raw)| {
            let Ok(kind) = CommandKind::from_str(&command) else {
                tracing::warn!(locale, command, "unknown command ignored");
                return None;
            };

            let entry = kind.entry();
            for parameter in raw.parameters.keys() {
                if !entry.has_param(parameter) {
                    tracing::warn!(
                        locale,
                        command,
                        parameter,
                        "unknown command parameter ignored"
                    );
                }
            }

            // Discord rejects the whole command registration over an oversized
            // description, so drop the override and fall back rather than fail startup.
            if let Some(short) = &raw.short_description
                && short.chars().count() > SHORT_DESCRIPTION_LIMIT
            {
                tracing::warn!(
                    locale,
                    command,
                    length = short.chars().count(),
                    limit = SHORT_DESCRIPTION_LIMIT,
                    "short_description exceeds Discord's limit, ignored"
                );
                raw.short_description = None;
            }

            Some((kind, raw))
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;

    use crate::models::command::registry;

    /// Look a command up by name, the way a help file key is resolved.
    fn kind(name: &str) -> CommandKind {
        CommandKind::from_str(name).expect("should be a registered command")
    }

    /// The shipped example config must parse and resolve, with locale overrides
    /// winning over the default-locale text.
    #[test]
    fn loads_example_help_and_resolves_locales() {
        let config = HelpConfig::load("help.example.toml").expect("example help should parse");

        let en = config.resolve(None, kind("play"));
        assert_eq!(en.short_description, "Start playback.");
        assert!(en.parameters.iter().any(|(name, _)| *name == "url"));

        let zh = config.resolve(Some("zh-TW"), kind("play"));
        assert_eq!(zh.short_description, "開始播放");
        let url_desc = zh
            .parameters
            .get(&"url")
            .expect("play should have a `url` parameter");
        assert_eq!(*url_desc, "可選參數，要播放的連結");
    }

    /// The example config is an *override* sample: the English text lives in the
    /// binary, so the example must not restate it. A `[default.<cmd>]` section here
    /// would be a second copy free to drift from the registered `CommandEntry`.
    #[test]
    fn example_help_does_not_restate_the_defaults() {
        let config = HelpConfig::load("help.example.toml").expect("example help should parse");

        for kind in registry().iter().copied() {
            let entry = kind.entry();
            let resolved = config.resolve(None, kind);

            assert_eq!(
                resolved.short_description, entry.short_description,
                "`{kind}` short_description is overridden in help.example.toml"
            );
            assert_eq!(
                resolved.description, entry.description,
                "`{kind}` description is overridden in help.example.toml"
            );
            for param in entry.parameters {
                assert_eq!(
                    resolved.parameters.get(param.name),
                    Some(&param.description),
                    "`{kind}.{}` is overridden in help.example.toml",
                    param.name
                );
            }
        }
    }

    /// Every `short_description` Discord will see must fit its 100 character limit,
    /// or registering the command fails at startup.
    #[test]
    fn example_short_descriptions_fit_discord_limit() {
        let config = HelpConfig::load("help.example.toml").expect("example help should parse");

        for locale in std::iter::once(None).chain(DISCORD_LOCALES.iter().copied().map(Some)) {
            for kind in registry().iter().copied() {
                let resolved = config.resolve(locale, kind);
                let len = resolved.short_description.chars().count();
                assert!(
                    len <= SHORT_DESCRIPTION_LIMIT,
                    "`{kind}` short_description is {len} characters for locale {locale:?}"
                );
            }
        }
    }

    /// A top-level key that isn't `default` and isn't a locale Discord knows is a typo,
    /// not a translation. It must be dropped, not registered.
    #[test]
    fn unknown_locale_is_dropped() {
        let raw: RawHelpConfig = toml::from_str(
            r#"
            [defualt.play]
            short_description = "typo'd `default`"

            [zh-TW.play]
            short_description = "real locale"
            "#,
        )
        .expect("should parse");
        let config = HelpConfig::from_raw(raw);

        assert!(config.locales.contains_key("zh-TW"));
        assert!(!config.locales.contains_key("defualt"));

        // The typo'd section must not leak into the default locale either.
        assert_eq!(
            config
                .resolve(Some("defualt"), kind("play"))
                .short_description,
            kind("play").entry().short_description
        );
    }

    /// An override longer than Discord allows is dropped in favour of the built-in
    /// text, so an edited help file can't stop the bot from starting.
    #[test]
    fn oversized_short_description_falls_back() {
        let long = "a".repeat(SHORT_DESCRIPTION_LIMIT + 1);
        let raw: RawHelpConfig = toml::from_str(&format!(
            r#"
            [default.play]
            short_description = "{long}"
            description = "kept"
            "#
        ))
        .expect("should parse");
        let config = HelpConfig::from_raw(raw);
        let resolved = config.resolve(None, kind("play"));

        assert_eq!(
            resolved.short_description,
            kind("play").entry().short_description
        );
        // Only the offending field is dropped.
        assert_eq!(resolved.description, "kept");
    }

    /// A field absent from a locale falls back to the default-locale value, then to
    /// the registered `CommandEntry` default.
    #[test]
    fn missing_locale_falls_back() {
        let config = HelpConfig::default();
        let resolved = config.resolve(Some("zh-TW"), kind("play"));
        assert_eq!(
            resolved.short_description,
            kind("play").entry().short_description
        );
    }

    /// A `hide_in_help` command is not a `/help` choice — but its short description is
    /// still registered with Discord and still localizable, so it must remain
    /// resolvable by name.
    #[test]
    fn command_hidden_from_help_still_resolves() {
        let help = kind("help");
        assert!(help.entry().hide_in_help);

        let config = HelpConfig::load("help.example.toml").expect("example help should parse");
        let zh = config.resolve(Some("zh-TW"), help);
        assert_eq!(zh.short_description, "查詢指令的詳細用法");
        assert_eq!(zh.description, help.entry().description);
    }
}
