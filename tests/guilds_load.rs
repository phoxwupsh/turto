//! Loading `guilds.json` must survive bad data in it.
//!
//! `Guilds::load` failing is not a contained error: the caller falls back to an empty
//! map and writes that back over the file at shutdown, so anything that fails the whole
//! deserialization costs every guild its queue, bans and settings.

use turto::models::guild::Guilds;

/// A stored volume outside `0.0..=1.0` -- hand-edited, or written by an older build --
/// is clamped, and the rest of the file loads intact.
#[test]
fn an_out_of_range_stored_volume_is_clamped_without_losing_the_file() {
    let file = tempfile::NamedTempFile::new().expect("temp file");
    std::fs::write(
        file.path(),
        r#"{
            "123456789012345678": {
                "config": {"auto_leave":"On","repeat":false,"volume":500.0,"banned":[]},
                "playlist": []
            },
            "987654321098765432": {
                "config": {
                    "auto_leave":"On","repeat":true,"volume":-0.5,
                    "banned":["111111111111111111"]
                },
                "playlist": []
            }
        }"#,
    )
    .expect("write fixture");

    let guilds = Guilds::load(file.path()).expect("one bad volume must not fail the whole load");

    assert_eq!(guilds.len(), 2, "both guilds must survive the load");
    for entry in guilds.iter() {
        let volume = *entry.config.volume;
        assert!(
            (0.0..=1.0).contains(&volume),
            "stored volume {volume} escaped the valid range"
        );
    }

    let repeat_guild = guilds
        .iter()
        .find(|entry| entry.config.repeat)
        .expect("the second guild's settings must survive");
    assert_eq!(
        repeat_guild.config.banned.len(),
        1,
        "the ban list must survive alongside the clamped volume"
    );
}
