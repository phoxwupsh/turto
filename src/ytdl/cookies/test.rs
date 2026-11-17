use super::*;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

const SENTINEL: &str = "SENTINEL_SECRET_9f3c";

/// The magic line alone, as browser exporters and yt-dlp start a file;
/// fixtures use it so parsing never depends on turto's own notice.
const HEADER: &str = "# Netscape HTTP Cookie File\n";

fn line(domain: &str, name: &str, value: &str) -> String {
    format!("{domain}\tTRUE\t/\tTRUE\t1900000000\t{name}\t{value}\n")
}

fn file(lines: &[String]) -> String {
    let mut out = HEADER.to_owned();
    for l in lines {
        out.push_str(l);
    }
    out
}

fn jar(lines: &[String]) -> Jar {
    let (jar, skipped) = parse(file(lines).as_bytes());
    assert_eq!(skipped, 0);
    jar
}

fn key(domain: &str, name: &str) -> Key {
    Key {
        domain: domain.to_owned(),
        path: "/".to_owned(),
        name: name.to_owned(),
    }
}

fn value_of<'a>(jar: &'a Jar, domain: &str, name: &str) -> Option<&'a str> {
    jar.cookies
        .get(&key(domain, name))
        .map(|c| c.value.as_str())
}

fn allow_youtube() -> Allowlist {
    Allowlist::from_jar(&jar(&[line(".youtube.com", "A", "a")]))
}

// ----- S1: parsing / serialization -----

#[test]
fn round_trip() {
    let text = file(&[
        line(".youtube.com", "A", "a"),
        format!("{HTTPONLY_PREFIX}.youtube.com\tTRUE\t/\tTRUE\t\tB\tb\n"),
        "www.youtube.com\tFALSE\t/watch\tFALSE\t0\t\tnameless\n".to_owned(),
    ]);
    let (parsed, skipped) = parse(text.as_bytes());
    assert_eq!(skipped, 0);
    assert_eq!(parsed.len(), 3);
    let (bytes, refused) = parsed.serialize();
    assert_eq!(refused, 0);
    let (again, skipped) = parse(&bytes);
    assert_eq!(skipped, 0);
    assert!(again == parsed);
    // Deterministic: serializing again is byte-identical.
    assert_eq!(again.serialize().0, bytes);
    assert!(bytes.starts_with(MANAGED_HEADER.as_bytes()));
}

#[test]
fn managed_header_keeps_the_magic_line_and_parses_as_comments() {
    assert!(MANAGED_HEADER.starts_with(HEADER));
    assert_eq!(MANAGED_HEADER.lines().count(), 2);
    assert!(MANAGED_HEADER.lines().all(|l| l.starts_with('#')));
    let (empty, skipped) = parse(MANAGED_HEADER.as_bytes());
    assert_eq!((empty.len(), skipped), (0, 0));
    // A file with the magic line only parses to the same jar as turto's
    // own serialization of it.
    let parsed = jar(&[line(".youtube.com", "A", "a")]);
    let (bytes, _) = parsed.serialize();
    assert!(parse(&bytes).0 == parsed);
    assert_eq!(parse(&bytes).1, 0);
}

#[test]
fn httponly_prefix_is_accepted_stripped_and_not_written_back() {
    let parsed = jar(&[format!(
        "{HTTPONLY_PREFIX}.youtube.com\tTRUE\t/\tTRUE\t1900000000\tSID\tv\n"
    )]);
    assert_eq!(value_of(&parsed, ".youtube.com", "SID"), Some("v"));
    let text = String::from_utf8(parsed.serialize().0).unwrap();
    assert!(text.contains("\n.youtube.com\tTRUE\t/\tTRUE\t1900000000\tSID\tv\n"));
    assert!(!text.contains(HTTPONLY_PREFIX));
    // The prefix does not relax the line checks.
    let bad = format!("{HEADER}{HTTPONLY_PREFIX}.youtube.com\ttrue\t/\tTRUE\t1\tA\ta\n");
    assert_eq!(parse(bad.as_bytes()).1, 1);
}

/// An operator file in browser-exporter form: an `#HttpOnly_` line, a
/// session cookie with an empty expiry, and subdomain flags that disagree
/// with the leading-dot rule.
const OPERATOR_FILE: &str = "# Netscape HTTP Cookie File\n\
    #HttpOnly_.youtube.com\tTRUE\t/\tTRUE\t1900000000\t__Secure-1PSIDTS\tOLDVAL\n\
    .youtube.com\tTRUE\t/\tTRUE\t\tSESS\ts\n\
    www.youtube.com\tTRUE\t/\tFALSE\t1900000000\tPREF\tp\n\
    .youtube.com\tFALSE\t/\tTRUE\t1900000000\tLOGIN\tl\n";

/// What yt-dlp's load -> save round trip writes for [`OPERATOR_FILE`]
/// (`#HttpOnly_` dropped, empty expiry as `0`, flag from the leading dot),
/// with the PSIDTS value replaced by `psidts`.
fn ytdlp_rewrite(psidts: &str) -> String {
    format!(
        "# Netscape HTTP Cookie File\n\
        .youtube.com\tTRUE\t/\tTRUE\t1900000000\t__Secure-1PSIDTS\t{psidts}\n\
        .youtube.com\tTRUE\t/\tTRUE\t0\tSESS\ts\n\
        www.youtube.com\tFALSE\t/\tFALSE\t1900000000\tPREF\tp\n\
        .youtube.com\tTRUE\t/\tTRUE\t1900000000\tLOGIN\tl\n"
    )
}

fn parse_clean(text: &str) -> Jar {
    let (jar, skipped) = parse(text.as_bytes());
    assert_eq!(skipped, 0);
    jar
}

#[test]
fn ytdlp_rewrite_without_a_real_change_gives_an_empty_delta() {
    let base = parse_clean(OPERATOR_FILE);
    assert_eq!(base.len(), 4);
    assert!(base.diff(&parse_clean(&ytdlp_rewrite("OLDVAL"))).is_empty());
}

#[test]
fn serialized_operator_file_is_canonical_and_stable() {
    let parsed = parse_clean(OPERATOR_FILE);
    let (bytes, refused) = parsed.serialize();
    assert_eq!(refused, 0);
    let text = String::from_utf8(bytes).unwrap();
    assert!(!text.contains(HTTPONLY_PREFIX));
    assert!(text.contains("\n.youtube.com\tTRUE\t/\tTRUE\t0\tSESS\ts\n"));
    assert!(text.contains("\nwww.youtube.com\tFALSE\t/\tFALSE\t1900000000\tPREF\tp\n"));
    assert!(text.contains("\n.youtube.com\tTRUE\t/\tTRUE\t1900000000\tLOGIN\tl\n"));
    let again = parse_clean(&text);
    assert!(again == parsed);
    assert!(again == parse_clean(&ytdlp_rewrite("OLDVAL")));
}

#[test]
fn stale_ytdlp_answer_does_not_revert_a_newer_rotation() {
    let base = parse_clean(OPERATOR_FILE);
    let rotated = parse_clean(&ytdlp_rewrite("NEWVAL"));
    let stale = parse_clean(&ytdlp_rewrite("OLDVAL"));
    let allow = Allowlist::from_jar(&base);
    let mut current = base.clone();
    for result in [&rotated, &stale] {
        let filtered = filter_delta(base.diff(result), &allow);
        let Ok(next) = apply_capped(&current, &filtered.delta) else {
            panic!("cap rejected a same-size update");
        };
        current = next;
    }
    assert_eq!(
        value_of(&current, ".youtube.com", "__Secure-1PSIDTS"),
        Some("NEWVAL")
    );
    // Nothing else changed: the jar is exactly the rotated one.
    assert!(current == rotated);
}

#[test]
fn expiry_is_canonicalized_like_python_int_float() {
    // Expected values checked against `int(float(x))` in yt-dlp's Python.
    let table = [
        ("", "0"),
        ("0", "0"),
        ("00", "0"),
        ("0.9", "0"),
        ("1900000000", "1900000000"),
        ("01900000000", "1900000000"),
        ("1900000000.5", "1900000000"),
        ("1900000000.000", "1900000000"),
        ("4.99999999999999999999", "5"),
        ("9007199254740993", "9007199254740992"),
        ("18446744073709551616", "18446744073709551616"),
        ("99999999999999999999", "100000000000000000000"),
        (
            "123456789012345678901234567890.999",
            "123456789012345677877719597056",
        ),
    ];
    for (i, (field, want)) in table.into_iter().enumerate() {
        assert_eq!(canonical_expiry(field).as_deref(), Some(want), "case {i}");
    }
}

#[test]
fn expiry_outside_ytdlps_grammar_or_overflowing_is_rejected() {
    let huge = "9".repeat(400);
    let rejected = [
        "1e9",
        "+5",
        "-1",
        "1.",
        ".5",
        ".",
        "1.2.3",
        "\u{661}\u{662}",
        " 5",
        "5 ",
        "inf",
        "NaN",
        "0x10",
        "1_000",
        &huge,
    ];
    for (i, field) in rejected.into_iter().enumerate() {
        assert!(canonical_expiry(field).is_none(), "case {i}");
        let input = format!("{HEADER}.youtube.com\tTRUE\t/\tTRUE\t{field}\tA\ta\n");
        assert_eq!(parse(input.as_bytes()).1, 1, "case {i}");
    }
}

/// [`OPERATOR_FILE`] with the PSIDTS expiry replaced by `expiry`.
fn operator_file_with_psidts_expiry(expiry: &str) -> String {
    let text = OPERATOR_FILE.replacen(
        "1900000000\t__Secure-1PSIDTS",
        &format!("{expiry}\t__Secure-1PSIDTS"),
        1,
    );
    assert_ne!(text, OPERATOR_FILE);
    text
}

#[test]
fn noncanonical_expiry_rewrite_gives_an_empty_delta_and_keeps_the_rotation() {
    for (i, expiry) in ["1900000000.5", "01900000000", "1900000000.000"]
        .into_iter()
        .enumerate()
    {
        // yt-dlp writes these back as `1900000000`, as `ytdlp_rewrite` does.
        let base = parse_clean(&operator_file_with_psidts_expiry(expiry));
        assert_eq!(base.len(), 4, "case {i}");
        assert!(
            base.diff(&parse_clean(&ytdlp_rewrite("OLDVAL"))).is_empty(),
            "case {i}"
        );
        let rotated = parse_clean(&ytdlp_rewrite("NEWVAL"));
        let stale = parse_clean(&ytdlp_rewrite("OLDVAL"));
        let allow = Allowlist::from_jar(&base);
        let mut current = base.clone();
        for result in [&rotated, &stale] {
            let filtered = filter_delta(base.diff(result), &allow);
            let Ok(next) = apply_capped(&current, &filtered.delta) else {
                panic!("cap rejected a same-size update");
            };
            current = next;
        }
        assert_eq!(
            value_of(&current, ".youtube.com", "__Secure-1PSIDTS"),
            Some("NEWVAL"),
            "case {i}"
        );
        assert!(current == rotated, "case {i}");
        // turto's own file holds the canonical expiry.
        let text = String::from_utf8(current.serialize().0).unwrap();
        assert!(
            text.contains("\t1900000000\t__Secure-1PSIDTS\t"),
            "case {i}"
        );
    }
}

#[test]
fn huge_expiry_is_stored_as_ytdlp_rounds_it() {
    let operator = operator_file_with_psidts_expiry("99999999999999999999");
    let rewrite = |v: &str| {
        ytdlp_rewrite(v).replacen(
            "1900000000\t__Secure-1PSIDTS",
            "100000000000000000000\t__Secure-1PSIDTS",
            1,
        )
    };
    let base = parse_clean(&operator);
    assert!(base.diff(&parse_clean(&rewrite("OLDVAL"))).is_empty());
}

#[test]
fn comments_blank_malformed_and_non_utf8_are_skipped_and_counted() {
    let mut input = Vec::new();
    input.extend_from_slice(HEADER.as_bytes());
    input.extend_from_slice(b"# a comment\n\n");
    input.extend_from_slice(line(".youtube.com", "A", "a").as_bytes());
    input.extend_from_slice(b"too\tfew\tfields\n"); // malformed
    input.extend_from_slice(b".youtube.com\tTRUE\t/\tTRUE\t1\tB\t\xff\xfe\n"); // non-UTF-8
    input.extend_from_slice(b"   \n"); // whitespace-only is not blank
    let (parsed, skipped) = parse(&input);
    assert_eq!(parsed.len(), 1);
    assert_eq!(skipped, 3);
}

#[test]
fn crlf_line_endings() {
    let text = file(&[
        line(".youtube.com", "A", "a"),
        line(".youtube.com", "B", "b"),
    ])
    .replace('\n', "\r\n");
    let (parsed, skipped) = parse(text.as_bytes());
    assert_eq!(skipped, 0);
    assert_eq!(value_of(&parsed, ".youtube.com", "A"), Some("a"));
    assert_eq!(value_of(&parsed, ".youtube.com", "B"), Some("b"));
}

#[test]
fn leading_space_or_tab_line_is_skipped() {
    let input = format!(
        "{HEADER} {}\t{}{}",
        line(".youtube.com", "A", "a"),
        line(".youtube.com", "B", "b"),
        line(".youtube.com", "C", "c")
    );
    let (parsed, skipped) = parse(input.as_bytes());
    assert_eq!(skipped, 2);
    assert_eq!(parsed.len(), 1);
    assert_eq!(value_of(&parsed, ".youtube.com", "C"), Some("c"));
}

#[test]
fn bad_flags_expiry_domain_and_control_chars_are_skipped() {
    let bad = [
        ".youtube.com\ttrue\t/\tTRUE\t1\tA\ta\n",
        ".youtube.com\tTRUE\t/\tyes\t1\tA\ta\n",
        ".youtube.com\tTRUE\t/\tTRUE\t-1\tA\ta\n",
        ".youtube.com\tTRUE\t/\tTRUE\t1e9\tA\ta\n",
        ".youtube.com\tTRUE\t/\tTRUE\t1.\tA\ta\n",
        "you tube.com\tTRUE\t/\tTRUE\t1\tA\ta\n",
        "youtube.com/\tTRUE\t/\tTRUE\t1\tA\ta\n",
        ".\tTRUE\t/\tTRUE\t1\tA\ta\n",
        "\tTRUE\t/\tTRUE\t1\tA\ta\n",
        ".youtube.com\tTRUE\t/\tTRUE\t1\tA\ta\u{7}b\n",
        ".youtube.com\tTRUE\t/\tTRUE\t1\tA\u{1b}\ta\n",
        ".youtube.com\tTRUE\t/\u{7f}\tTRUE\t1\tA\ta\n",
        ".youtube.com\tTRUE\t/\tTRUE\t1\tA\ta\tb\n",
        ".youtube.com\tTRUE\t/\tTRUE\t1\tA\ta\rb\n",
    ];
    let input = format!("{HEADER}{}", bad.concat());
    let (parsed, skipped) = parse(input.as_bytes());
    assert_eq!(parsed.len(), 0);
    assert_eq!(skipped, bad.len());
}

#[test]
fn serializer_refuses_cookies_with_line_breaking_chars() {
    let mut j = jar(&[line(".youtube.com", "A", "a")]);
    for (i, bad) in ["x\ty", "x\ry", "x\ny"].iter().enumerate() {
        j.cookies.insert(
            key(".youtube.com", &format!("BAD{i}")),
            Cookie {
                include_subdomains: true,
                secure: true,
                expiry: String::new(),
                value: (*bad).to_owned(),
            },
        );
    }
    j.cookies.insert(
        Key {
            domain: ".youtube.com".to_owned(),
            path: "/\n.evil.com".to_owned(),
            name: "P".to_owned(),
        },
        j.cookies.get(&key(".youtube.com", "A")).unwrap().clone(),
    );
    let (bytes, refused) = j.serialize();
    assert_eq!(refused, 4);
    let (back, skipped) = parse(&bytes);
    assert_eq!(skipped, 0);
    assert_eq!(back.len(), 1);
}

#[test]
fn domain_case_is_folded() {
    let parsed = jar(&[
        line(".YouTube.COM", "A", "first"),
        line(".youtube.com", "A", "second"),
    ]);
    assert_eq!(parsed.len(), 1);
    assert_eq!(value_of(&parsed, ".youtube.com", "A"), Some("second"));
}

// ----- S1: diff / apply -----

#[test]
fn out_of_order_updates_merge() {
    let v1 = jar(&[line(".youtube.com", "X", "x1")]);
    // B (from v1) changes X; applied first.
    let b_result = jar(&[line(".youtube.com", "X", "x2")]);
    let mut current = v1.clone();
    current.apply(&v1.diff(&b_result));
    assert_eq!(value_of(&current, ".youtube.com", "X"), Some("x2"));
    // A (also from v1) adds Y; applied to v2 it keeps B's X.
    let a_result = jar(&[
        line(".youtube.com", "X", "x1"),
        line(".youtube.com", "Y", "y"),
    ]);
    current.apply(&v1.diff(&a_result));
    assert_eq!(value_of(&current, ".youtube.com", "X"), Some("x2"));
    assert_eq!(value_of(&current, ".youtube.com", "Y"), Some("y"));
}

#[test]
fn absence_in_result_is_a_removal() {
    let base = jar(&[
        line(".youtube.com", "X", "x"),
        line(".youtube.com", "Y", "y"),
    ]);
    let result = jar(&[line(".youtube.com", "X", "x")]);
    let delta = base.diff(&result);
    assert_eq!(delta.removals.len(), 1);
    assert!(delta.upserts.is_empty());
    let mut current = base.clone();
    current.apply(&delta);
    assert!(current == result);
}

#[test]
fn identical_jars_give_an_empty_delta() {
    let base = jar(&[line(".youtube.com", "X", "x")]);
    assert!(base.diff(&base.clone()).is_empty());
}

#[test]
fn last_applied_wins_on_conflict() {
    let base = jar(&[line(".youtube.com", "X", "x0")]);
    let first = base.diff(&jar(&[line(".youtube.com", "X", "x1")]));
    let second = base.diff(&jar(&[line(".youtube.com", "X", "x2")]));
    let mut current = base.clone();
    current.apply(&first);
    current.apply(&second);
    assert_eq!(value_of(&current, ".youtube.com", "X"), Some("x2"));
}

// ----- S1: allowlist / caps -----

#[test]
fn foreign_domain_upsert_is_dropped_not_removed() {
    let allow = allow_youtube();
    let base = jar(&[line(".youtube.com", "A", "a")]);
    let result = jar(&[line(".youtube.com", "A", "a"), line(".evil.com", "E", "e")]);
    let Filtered { delta, foreign, .. } = filter_delta(base.diff(&result), &allow);
    assert_eq!(foreign, 1);
    assert!(delta.is_empty());
    let mut current = base.clone();
    current.apply(&delta);
    assert!(current == base);
}

#[test]
fn subdomain_accepted_lookalike_rejected() {
    let allow = allow_youtube();
    assert!(allow.allows(".youtube.com"));
    assert!(allow.allows("youtube.com"));
    assert!(allow.allows("www.youtube.com"));
    assert!(allow.allows(".music.youtube.com"));
    assert!(!allow.allows("evilyoutube.com"));
    assert!(!allow.allows(".evilyoutube.com"));
    assert!(!allow.allows("youtube.com.evil.com"));
    assert!(!allow.allows("com"));
    assert!(!allow.allows("."));
    assert!(!allow.allows(""));
}

#[test]
fn empty_initial_jar_accepts_nothing() {
    let allow = Allowlist::from_jar(&Jar::default());
    let result = jar(&[line(".youtube.com", "A", "a")]);
    let filtered = filter_delta(Jar::default().diff(&result), &allow);
    assert!(filtered.delta.is_empty());
    assert_eq!(filtered.foreign, 1);
}

#[test]
fn oversized_cookie_removed_only_by_genuine_absence() {
    let allow = allow_youtube();
    let big = "v".repeat(MAX_COOKIE_LINE + 1);
    let base = jar(&[
        line(".youtube.com", "A", "a"),
        line(".youtube.com", "BIG", &big),
    ]);

    // A result that omits the big cookie is a genuine removal.
    let result = jar(&[line(".youtube.com", "A", "a")]);
    let filtered = filter_delta(base.diff(&result), &allow);
    assert_eq!(filtered.delta.removals.len(), 1);
    assert_eq!(filtered.oversized, 0);

    // An oversized upsert of the same key is dropped and the cookie kept.
    let bigger = "w".repeat(MAX_COOKIE_LINE + 10);
    let result = jar(&[
        line(".youtube.com", "A", "a"),
        line(".youtube.com", "BIG", &bigger),
    ]);
    let filtered = filter_delta(base.diff(&result), &allow);
    assert_eq!(filtered.oversized, 1);
    assert!(filtered.delta.is_empty());
    let mut current = base.clone();
    current.apply(&filtered.delta);
    assert_eq!(
        value_of(&current, ".youtube.com", "BIG"),
        Some(big.as_str())
    );
}

#[test]
fn growth_past_the_total_cap_is_rejected_whole() {
    let base = jar(&[line(".youtube.com", "A", "a")]);
    let mut lines = vec![line(".youtube.com", "A", "changed")];
    lines.extend((0..MAX_COOKIES).map(|i| line(".youtube.com", &format!("N{i}"), "n")));
    let result = jar(&lines);
    let filtered = filter_delta(base.diff(&result), &allow_youtube());
    assert!(apply_capped(&base, &filtered.delta).is_err());

    // Byte cap: a few large cookies past MAX_JAR_BYTES.
    let chunk = "v".repeat(4000);
    let many: Vec<String> = (0..(MAX_JAR_BYTES / 4000 + 2))
        .map(|i| line(".youtube.com", &format!("L{i}"), &chunk))
        .collect();
    let filtered = filter_delta(base.diff(&jar(&many)), &allow_youtube());
    assert!(apply_capped(&base, &filtered.delta).is_err());
}

#[test]
fn jar_over_the_count_cap_still_accepts_a_value_change() {
    let lines: Vec<String> = (0..MAX_COOKIES + 5)
        .map(|i| line(".youtube.com", &format!("N{i}"), "old"))
        .collect();
    let base = jar(&lines);
    let mut changed = base.clone();
    changed
        .cookies
        .get_mut(&key(".youtube.com", "N3"))
        .unwrap()
        .value = "new".to_owned();
    let filtered = filter_delta(base.diff(&changed), &Allowlist::from_jar(&base));
    let Ok(next) = apply_capped(&base, &filtered.delta) else {
        panic!("a value change must be accepted on an over-cap jar");
    };
    assert_eq!(value_of(&next, ".youtube.com", "N3"), Some("new"));
    assert_eq!(next.len(), base.len());
}

#[test]
fn debug_never_shows_cookie_data() {
    let base = jar(&[line(".youtube.com", SENTINEL, SENTINEL)]);
    let delta = Jar::default().diff(&base);
    let snap = Snapshot {
        jar: base.clone(),
        b64: BASE64_STANDARD.encode(SENTINEL),
    };
    let msg = Msg::Update {
        base: Arc::new(Snapshot {
            jar: base.clone(),
            b64: String::new(),
        }),
        result: base.clone(),
    };
    let encoded = BASE64_STANDARD.encode(SENTINEL);
    for text in [
        format!("{base:?}"),
        format!("{delta:?}"),
        format!("{snap:?}"),
        format!("{msg:?}"),
    ] {
        assert!(!text.contains(SENTINEL), "{text}");
        assert!(!text.contains(&encoded), "{text}");
    }
}

// ----- S3: store / updater -----

/// A sink that records each save and fails the first `fail_first` of them.
#[derive(Clone, Default)]
struct Recorder {
    calls: Arc<AtomicUsize>,
    saved: Arc<Mutex<Option<Vec<u8>>>>,
    fail_first: usize,
}

impl Sink for Recorder {
    fn save(&mut self, bytes: &[u8]) -> SaveOutcome {
        let n = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        if n <= self.fail_first {
            return SaveOutcome::Failed;
        }
        *self.saved.lock().unwrap() = Some(bytes.to_vec());
        SaveOutcome::Saved
    }
}

const LONG: Duration = Duration::from_secs(3600);
const FLUSH: Duration = Duration::from_secs(5);

fn store_with<S: Sink>(initial: &[String], debounce: Duration, sink: S) -> CookieStore {
    let text = file(initial);
    let (store, updater) =
        CookieStore::with_sink(jar(initial), BASE64_STANDARD.encode(&text), debounce, sink);
    tokio::spawn(updater);
    store
}

async fn wait_until(mut cond: impl FnMut() -> bool) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !cond() {
        assert!(
            std::time::Instant::now() < deadline,
            "condition not met in time"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

#[tokio::test]
async fn two_updates_from_the_same_base_merge() {
    let initial = [
        line(".youtube.com", "X", "x1"),
        line(".youtube.com", "Y", "y1"),
    ];
    let store = store_with(&initial, LONG, Recorder::default());
    let base = store.load();
    store.submit(
        base.clone(),
        jar(&[
            line(".youtube.com", "X", "x2"),
            line(".youtube.com", "Y", "y1"),
        ]),
    );
    store.submit(
        base,
        jar(&[
            line(".youtube.com", "X", "x1"),
            line(".youtube.com", "Y", "y2"),
        ]),
    );
    store.flush(FLUSH).await;
    let now = store.load();
    assert_eq!(value_of(&now.jar, ".youtube.com", "X"), Some("x2"));
    assert_eq!(value_of(&now.jar, ".youtube.com", "Y"), Some("y2"));
    // The new snapshot carries the merged jar as its base64.
    let decoded = BASE64_STANDARD.decode(&now.b64).unwrap();
    assert!(parse(&decoded).0 == now.jar);
}

#[tokio::test]
async fn stale_answer_through_the_store_keeps_the_rotation_on_disk() {
    let initial: Vec<String> = OPERATOR_FILE
        .lines()
        .skip(1)
        .map(|l| format!("{l}\n"))
        .collect();
    let recorder = Recorder::default();
    let store = store_with(&initial, LONG, recorder.clone());
    let base = store.load();
    store.submit(base.clone(), parse_clean(&ytdlp_rewrite("NEWVAL")));
    store.submit(base, parse_clean(&ytdlp_rewrite("OLDVAL")));
    store.flush(FLUSH).await;
    let rotated = parse_clean(&ytdlp_rewrite("NEWVAL"));
    assert!(store.load().jar == rotated);
    let saved = recorder.saved.lock().unwrap().clone().unwrap();
    assert!(parse(&saved).0 == rotated);
}

#[tokio::test]
async fn flush_writes_a_file_that_parses_back_to_the_current_jar() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cookies.txt");
    let initial = [line(".youtube.com", "X", "x1")];
    let text = file(&initial);
    std::fs::write(&path, &text).unwrap();
    let (store, updater) = CookieStore::new(path.clone(), jar(&initial), text.into_bytes(), LONG);
    tokio::spawn(updater);

    store.submit(store.load(), jar(&[line(".youtube.com", "X", "x2")]));
    store.flush(FLUSH).await;
    let on_disk = std::fs::read(&path).unwrap();
    assert!(parse(&on_disk).0 == store.load().jar);
    assert_eq!(
        value_of(&parse(&on_disk).0, ".youtube.com", "X"),
        Some("x2")
    );
}

#[tokio::test]
async fn empty_or_filtered_deltas_do_not_write() {
    let recorder = Recorder::default();
    let initial = [line(".youtube.com", "X", "x1")];
    let store = store_with(&initial, Duration::from_millis(10), recorder.clone());
    // Same jar back, and a foreign-only change.
    store.submit(store.load(), jar(&initial));
    store.submit(
        store.load(),
        jar(&[line(".youtube.com", "X", "x1"), line(".evil.com", "E", "e")]),
    );
    store.flush(FLUSH).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(recorder.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn dropped_updater_does_not_make_the_producer_panic() {
    let initial = [line(".youtube.com", "X", "x1")];
    let (store, updater) =
        CookieStore::with_sink(jar(&initial), String::new(), LONG, Recorder::default());
    drop(updater);
    store.submit(store.load(), jar(&[line(".youtube.com", "X", "x2")]));
    store.flush(Duration::from_millis(100)).await;
    assert_eq!(value_of(&store.load().jar, ".youtube.com", "X"), Some("x1"));
}

#[tokio::test]
async fn update_is_persisted_after_the_debounce_deadline() {
    let recorder = Recorder::default();
    let store = store_with(
        &[line(".youtube.com", "X", "x1")],
        Duration::from_millis(20),
        recorder.clone(),
    );
    store.submit(store.load(), jar(&[line(".youtube.com", "X", "x2")]));
    wait_until(|| recorder.calls.load(Ordering::SeqCst) == 1).await;
    let saved = recorder.saved.lock().unwrap().clone().unwrap();
    assert!(parse(&saved).0 == store.load().jar);
}

#[tokio::test]
async fn updates_inside_one_window_produce_one_write() {
    let recorder = Recorder::default();
    let store = store_with(
        &[line(".youtube.com", "X", "x0")],
        Duration::from_millis(200),
        recorder.clone(),
    );
    for i in 1..=5 {
        store.submit(
            store.load(),
            jar(&[line(".youtube.com", "X", &format!("x{i}"))]),
        );
    }
    wait_until(|| recorder.calls.load(Ordering::SeqCst) >= 1).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(recorder.calls.load(Ordering::SeqCst), 1);
    let saved = recorder.saved.lock().unwrap().clone().unwrap();
    assert_eq!(value_of(&parse(&saved).0, ".youtube.com", "X"), Some("x5"));
}

#[tokio::test]
async fn failed_save_is_retried_at_the_next_deadline() {
    let recorder = Recorder {
        fail_first: 1,
        ..Recorder::default()
    };
    let store = store_with(
        &[line(".youtube.com", "X", "x1")],
        Duration::from_millis(20),
        recorder.clone(),
    );
    store.submit(store.load(), jar(&[line(".youtube.com", "X", "x2")]));
    wait_until(|| recorder.calls.load(Ordering::SeqCst) == 2).await;
    let saved = recorder.saved.lock().unwrap().clone().unwrap();
    assert!(parse(&saved).0 == store.load().jar);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(
        recorder.calls.load(Ordering::SeqCst),
        2,
        "no saves once clean"
    );
}

// ----- S4: persistence -----

struct Target {
    _dir: tempfile::TempDir,
    dir: PathBuf,
    path: PathBuf,
    original: Vec<u8>,
}

fn target(content: &[u8]) -> Target {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cookies.txt");
    std::fs::write(&path, content).unwrap();
    Target {
        dir: dir.path().to_owned(),
        _dir: dir,
        path,
        original: content.to_vec(),
    }
}

fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn new_bytes() -> Vec<u8> {
    jar(&[line(".youtube.com", "X", "new")]).serialize().0
}

#[test]
fn atomic_path_writes_and_leaves_no_temp_file() {
    let t = target(file(&[line(".youtube.com", "X", "old")]).as_bytes());
    let seen: Arc<Mutex<Option<PathBuf>>> = Arc::default();
    let mut p = Persister::new(t.path.clone());
    let record = seen.clone();
    p.temp_writer = Box::new(move |tmp, bytes| {
        *record.lock().unwrap() = Some(tmp.path().to_owned());
        tmp.write_all(bytes)
    });
    assert!(matches!(p.save(&new_bytes()), SaveOutcome::Saved));
    assert_eq!(std::fs::read(&t.path).unwrap(), new_bytes());
    assert_eq!(entries(&t.dir), ["cookies.txt"]);

    let tmp = seen.lock().unwrap().clone().expect("atomic path used");
    assert_eq!(tmp.parent().unwrap(), t.dir.canonicalize().unwrap());
    let name = tmp.file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.starts_with(".cookies.txt.turto-tmp-"), "{name}");
    assert_eq!(temp_prefix(&t.path), ".cookies.txt.turto-tmp-");
}

#[test]
fn in_place_fallback_truncates_a_longer_file() {
    let long = file(&[line(".youtube.com", "X", &"old".repeat(200))]);
    let t = target(long.as_bytes());
    let mut p = Persister::new(t.path.clone());
    p.atomic_disabled = true;
    p.temp_writer = Box::new(|_, _| panic!("atomic path must not be used"));
    assert!(matches!(p.save(&new_bytes()), SaveOutcome::Saved));
    assert_eq!(std::fs::read(&t.path).unwrap(), new_bytes());
}

#[test]
fn externally_modified_target_is_overwritten() {
    let t = target(file(&[line(".youtube.com", "X", "old")]).as_bytes());
    let mut p = Persister::new(t.path.clone());
    let edited = file(&[line(".youtube.com", "X", "re-exported")]);
    std::fs::write(&t.path, &edited).unwrap();
    assert!(matches!(p.save(&new_bytes()), SaveOutcome::Saved));
    assert_eq!(std::fs::read(&t.path).unwrap(), new_bytes());
    assert_eq!(entries(&t.dir), ["cookies.txt"]);

    // Also through the store: outside edits never stop persisting.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let initial = [line(".youtube.com", "X", "old")];
        let (store, updater) =
            CookieStore::new(t.path.clone(), jar(&initial), t.original.clone(), LONG);
        tokio::spawn(updater);
        std::fs::write(&t.path, &edited).unwrap();
        store.submit(store.load(), jar(&[line(".youtube.com", "X", "v2")]));
        store.flush(FLUSH).await;
        let on_disk = parse(&std::fs::read(&t.path).unwrap()).0;
        assert_eq!(value_of(&on_disk, ".youtube.com", "X"), Some("v2"));
        std::fs::write(&t.path, &edited).unwrap();
        store.submit(store.load(), jar(&[line(".youtube.com", "X", "v3")]));
        store.flush(FLUSH).await;
        let on_disk = parse(&std::fs::read(&t.path).unwrap()).0;
        assert_eq!(value_of(&on_disk, ".youtube.com", "X"), Some("v3"));
    });
}

#[test]
fn missing_target_is_recreated() {
    let t = target(file(&[line(".youtube.com", "X", "old")]).as_bytes());
    std::fs::remove_file(&t.path).unwrap();
    let mut p = Persister::new(t.path.clone());
    assert!(matches!(p.save(&new_bytes()), SaveOutcome::Saved));
    assert_eq!(std::fs::read(&t.path).unwrap(), new_bytes());
    // No `.turto-tmp-` leftovers.
    assert_eq!(entries(&t.dir), ["cookies.txt"]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&t.path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    // Deleted again later: recreated again, not stopped.
    std::fs::remove_file(&t.path).unwrap();
    assert!(matches!(p.save(&new_bytes()), SaveOutcome::Saved));
    assert_eq!(std::fs::read(&t.path).unwrap(), new_bytes());
}

#[test]
fn missing_parent_directory_is_a_failed_save() {
    let t = target(file(&[line(".youtube.com", "X", "old")]).as_bytes());
    let path = t.dir.join("gone").join("cookies.txt");
    let mut p = Persister::new(path.clone());
    assert!(matches!(p.save(&new_bytes()), SaveOutcome::Failed));
    assert!(matches!(p.save(&new_bytes()), SaveOutcome::Failed));
    assert!(!path.exists());
    assert_eq!(entries(&t.dir), ["cookies.txt"]);
    // Once the directory is back, the retry recreates the file.
    std::fs::create_dir(t.dir.join("gone")).unwrap();
    assert!(matches!(p.save(&new_bytes()), SaveOutcome::Saved));
    assert_eq!(std::fs::read(&path).unwrap(), new_bytes());
}

#[test]
fn temp_write_failure_on_a_missing_target_creates_nothing() {
    let t = target(file(&[line(".youtube.com", "X", "old")]).as_bytes());
    std::fs::remove_file(&t.path).unwrap();
    let mut p = Persister::new(t.path.clone());
    p.temp_writer = Box::new(|_, _| Err(io::Error::other("injected write failure")));
    assert!(matches!(p.save(&new_bytes()), SaveOutcome::Failed));
    assert!(entries(&t.dir).is_empty());
}

#[test]
fn read_only_target_is_not_written() {
    let t = target(file(&[line(".youtube.com", "X", "old")]).as_bytes());
    let mut perms = std::fs::metadata(&t.path).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&t.path, perms).unwrap();
    let mut p = Persister::new(t.path.clone());
    assert!(matches!(p.save(&new_bytes()), SaveOutcome::Stopped));
    assert_eq!(std::fs::read(&t.path).unwrap(), t.original);
    assert_eq!(entries(&t.dir), ["cookies.txt"]);
}

#[test]
fn temp_write_failure_leaves_the_target_untouched() {
    let t = target(file(&[line(".youtube.com", "X", "old")]).as_bytes());
    let mut p = Persister::new(t.path.clone());
    p.temp_writer = Box::new(|tmp, bytes| {
        // Simulate a partial write, then ENOSPC.
        tmp.write_all(&bytes[..bytes.len() / 2])?;
        Err(io::Error::other("injected write failure"))
    });
    assert!(matches!(p.save(&new_bytes()), SaveOutcome::Failed));
    assert_eq!(std::fs::read(&t.path).unwrap(), t.original);
    assert_eq!(entries(&t.dir), ["cookies.txt"]);

    // The failure does not stop persisting: the next save goes through.
    p.temp_writer = Box::new(|tmp, bytes| tmp.write_all(bytes));
    assert!(matches!(p.save(&new_bytes()), SaveOutcome::Saved));
    assert_eq!(std::fs::read(&t.path).unwrap(), new_bytes());
}

#[cfg(unix)]
#[test]
fn unix_mode_is_preserved() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let t = target(file(&[line(".youtube.com", "X", "old")]).as_bytes());
    std::fs::set_permissions(&t.path, std::fs::Permissions::from_mode(0o640)).unwrap();
    let before = std::fs::metadata(&t.path).unwrap().ino();
    let mut p = Persister::new(t.path.clone());
    assert!(matches!(p.save(&new_bytes()), SaveOutcome::Saved));
    let meta = std::fs::metadata(&t.path).unwrap();
    assert_ne!(meta.ino(), before, "expected the atomic (rename) path");
    assert_eq!(meta.permissions().mode() & 0o777, 0o640);
}

#[cfg(unix)]
#[test]
fn unix_hard_linked_target_is_written_in_place() {
    use std::os::unix::fs::MetadataExt;
    let t = target(file(&[line(".youtube.com", "X", "old")]).as_bytes());
    let link = t.dir.join("linked.txt");
    std::fs::hard_link(&t.path, &link).unwrap();
    let before = std::fs::metadata(&t.path).unwrap().ino();
    let mut p = Persister::new(t.path.clone());
    p.temp_writer = Box::new(|_, _| panic!("atomic path must not be used"));
    assert!(matches!(p.save(&new_bytes()), SaveOutcome::Saved));
    assert_eq!(std::fs::metadata(&t.path).unwrap().ino(), before);
    assert_eq!(std::fs::read(&link).unwrap(), new_bytes());
    assert_eq!(std::fs::metadata(&link).unwrap().nlink(), 2);
}

/// Two real update+flush rounds against the real file: the second save
/// must land on top of turto's own first write.
async fn consecutive_saves(force_in_place: bool) {
    let initial = [line(".youtube.com", "X", "x1")];
    let t = target(file(&initial).as_bytes());
    let mut persister = Persister::new(t.path.clone());
    if force_in_place {
        persister.atomic_disabled = true;
        persister.temp_writer = Box::new(|_, _| panic!("atomic path must not be used"));
    }
    let (store, updater) = CookieStore::with_sink(
        jar(&initial),
        BASE64_STANDARD.encode(&t.original),
        LONG,
        persister,
    );
    tokio::spawn(updater);

    store.submit(store.load(), jar(&[line(".youtube.com", "X", "x2")]));
    store.flush(FLUSH).await;
    let first = parse(&std::fs::read(&t.path).unwrap()).0;
    assert_eq!(value_of(&first, ".youtube.com", "X"), Some("x2"));

    store.submit(
        store.load(),
        jar(&[
            line(".youtube.com", "X", "x3"),
            line(".youtube.com", "Y", "y"),
        ]),
    );
    store.flush(FLUSH).await;
    let second = parse(&std::fs::read(&t.path).unwrap()).0;
    assert!(second == store.load().jar);
    assert_eq!(value_of(&second, ".youtube.com", "X"), Some("x3"));
    assert_eq!(value_of(&second, ".youtube.com", "Y"), Some("y"));
    assert_eq!(entries(&t.dir), ["cookies.txt"]);
}

#[tokio::test]
async fn consecutive_real_saves_atomic() {
    consecutive_saves(false).await;
}

#[tokio::test]
async fn consecutive_real_saves_in_place() {
    consecutive_saves(true).await;
}
