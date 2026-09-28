//! Whether a newer dmm-tools release is on GitHub.
//!
//! Once a day a downloaded build asks GitHub's public releases API for the
//! releases it could move to, and the top bar links to the newest one. The
//! build the user runs decides which those are: a release hears of later
//! releases, a dev build of later dev builds too, and of the release that
//! supersedes its version. Source builds never ask: their commit matches no
//! published build, so every nightly would read as newer.
//!
//! The answer is cached beside the settings file, in a file of its own so a
//! long-running window writing it can't overwrite settings another window
//! changed. Only a validated tag name is taken from the response; the link is
//! built from the repository URL and that tag.
//!
//! Built without the `update-check` feature, no build asks, the request code
//! and its HTTP and TLS crates are left out, and the Settings row never shows.

use std::path::PathBuf;
use std::sync::mpsc;

use chrono::{DateTime, SubsecRound, TimeDelta, Utc};
use eframe::egui;
use serde::{Deserialize, Serialize};

/// Set by the workflows that publish binaries (`build-matrix.yml`'s
/// `published` input). Read at compile time, so cargo rebuilds when it changes.
fn published_build() -> bool {
    cfg!(feature = "update-check") && option_env!("DMM_PUBLISHED_BUILD") == Some("1")
}

const API: &str = "https://api.github.com/repos/antoinecellerier/dmm-tools";
const RELEASE_PAGE: &str = "https://github.com/antoinecellerier/dmm-tools/releases/tag/";

/// At most one request per day, whatever its outcome.
const INTERVAL: TimeDelta = TimeDelta::days(1);
/// A page of 30 holds the seven kept dev builds and the releases between them
/// many times over.
const PAGE_SIZE: u32 = 30;
/// A page of 30 releases with their notes is about 250 KB.
#[cfg(feature = "update-check")]
const BODY_LIMIT: u64 = 1 << 20;
#[cfg(feature = "update-check")]
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// A release this build could move to, from a validated tag name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Tag {
    /// `vX.Y.Z`.
    Release(Version),
    /// `dev-<hash>`: a nightly built from `main`.
    Dev(String),
}

type Version = (u32, u32, u32);

impl Tag {
    /// `v<u32>.<u32>.<u32>` or `dev-` and 7–40 lowercase hex digits; anything
    /// else is not one of ours.
    pub(crate) fn parse(tag: &str) -> Option<Self> {
        if let Some(hash) = tag.strip_prefix("dev-") {
            return is_hash(hash).then(|| Tag::Dev(hash.to_string()));
        }
        parse_version(tag.strip_prefix('v')?).map(Tag::Release)
    }

    /// The tag as GitHub names it.
    pub(super) fn name(&self) -> String {
        match self {
            Tag::Release((major, minor, patch)) => format!("v{major}.{minor}.{patch}"),
            Tag::Dev(hash) => format!("dev-{hash}"),
        }
    }

    pub(super) fn url(&self) -> String {
        format!("{RELEASE_PAGE}{}", self.name())
    }

    /// What the top bar says when it has room.
    pub(super) fn label(&self) -> String {
        match self {
            Tag::Release(_) => format!("{} available \u{2197}", self.name()),
            Tag::Dev(_) => "Newer dev build \u{2197}".to_string(),
        }
    }

    /// What it says when it is short of room.
    pub(super) fn short_label(&self) -> String {
        match self {
            Tag::Release(_) => format!("{} \u{2197}", self.name()),
            Tag::Dev(_) => "Update \u{2197}".to_string(),
        }
    }
}

fn is_hash(s: &str) -> bool {
    (7..=40).contains(&s.len()) && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// `X.Y.Z` with plain decimal parts — no sign, no suffix.
fn parse_version(s: &str) -> Option<Version> {
    let mut parts = s.split('.').map(|part| {
        (!part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
            .then(|| part.parse::<u32>().ok())
            .flatten()
    });
    let version = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(version)
}

/// One entry of the releases API, reduced to what the decision reads.
#[derive(Debug, Deserialize)]
pub(super) struct Release {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    published_at: Option<String>,
}

/// The newest release a build of `version` from commit `hash` could move to,
/// if any.
///
/// A release build takes the highest later release. A dev build of `X.Y.Z-dev`
/// takes a release of `X.Y.Z` or later first — it supersedes the whole line —
/// and otherwise the most recently published dev build that is not its own.
/// Published, not created: GitHub dates a release by its commit, which can run
/// backwards if `main` is ever rewritten.
pub(super) fn newer_release(version: &str, hash: &str, releases: &[Release]) -> Option<Tag> {
    let (base, dev) = match version.split_once('-') {
        Some((base, "dev")) => (base, true),
        None => (version, false),
        Some(_) => return None,
    };
    let own = parse_version(base)?;
    let valid = releases.iter().filter(|r| !r.draft).filter_map(|r| {
        let published = DateTime::parse_from_rfc3339(r.published_at.as_deref()?).ok()?;
        Some((Tag::parse(&r.tag_name)?, published.with_timezone(&Utc)))
    });

    let mut best_release: Option<Version> = None;
    let mut newest_dev: Option<(String, DateTime<Utc>)> = None;
    for (tag, published) in valid {
        match tag {
            Tag::Release(v) if v > own || (dev && v == own) => {
                best_release = best_release.max(Some(v));
            }
            Tag::Release(_) => {}
            Tag::Dev(other) => {
                if newest_dev.as_ref().is_none_or(|(_, at)| published > *at) {
                    newest_dev = Some((other, published));
                }
            }
        }
    }
    if let Some(v) = best_release {
        return Some(Tag::Release(v));
    }
    // A build whose hash isn't a commit ("dev" in debug builds, "unknown"
    // without git) can't tell its own nightly from a newer one.
    if !dev || !is_hash(hash) {
        return None;
    }
    let (other, _) = newest_dev?;
    // Either may be the longer abbreviation of the same commit.
    let same = other.starts_with(hash) || hash.starts_with(&other);
    (!same).then_some(Tag::Dev(other))
}

/// The last check's outcome, kept beside the settings file.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Cache {
    /// When the last attempt was made, whether or not it got an answer.
    checked_at: Option<DateTime<Utc>>,
    /// The version label of the build that made it: another build's answer
    /// says nothing about this one.
    checked_by: Option<String>,
    /// The tag it found, if any.
    available: Option<String>,
}

impl Cache {
    fn path() -> Option<PathBuf> {
        dmm_shared::config_path().map(|p| p.with_file_name("update-check.json"))
    }

    fn load() -> Self {
        Self::path()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|contents| serde_json::from_str(&contents).ok())
            .unwrap_or_default()
    }

    fn save(&self) {
        let Some(path) = Self::path() else { return };
        if let Ok(json) = serde_json::to_string_pretty(self)
            && let Err(e) = dmm_shared::write_atomic(&path, json.as_bytes())
        {
            log::warn!("failed to save {}: {e}", path.display());
        }
    }

    /// Whether this build should ask again at `now`. A clock set back to
    /// before the last attempt counts as due, so it can't silence the check
    /// until the clock catches up.
    fn is_due(&self, now: DateTime<Utc>, running: &str) -> bool {
        match self.checked_at {
            _ if self.checked_by.as_deref() != Some(running) => true,
            None => true,
            Some(at) => now < at || now - at >= INTERVAL,
        }
    }

    /// The tag it found, if this build found it.
    fn available_for(&self, running: &str) -> Option<Tag> {
        if self.checked_by.as_deref() != Some(running) {
            return None;
        }
        Tag::parse(self.available.as_deref()?)
    }
}

type Answer = Result<Option<Tag>, String>;

#[derive(Debug, Default)]
enum Mode {
    /// A source build, or a test: never asks, never shows.
    #[default]
    Off,
    /// A downloaded build, and its version label, kept to compare with the
    /// cache's.
    Live { cache: Cache, running: String },
    /// `--update-notice`: shows the given tag like a found one, but asks
    /// nothing and saves nothing.
    Forced,
}

/// The update check's state for one window.
#[derive(Debug, Default)]
pub(super) struct UpdateCheck {
    mode: Mode,
    available: Option<Tag>,
    pending: Option<mpsc::Receiver<Answer>>,
}

impl UpdateCheck {
    /// The check for a window: live in a downloaded build, forced to `notice`
    /// by the hidden flag, off otherwise.
    pub(super) fn new(notice: Option<Tag>) -> Self {
        if let Some(tag) = notice {
            return Self {
                mode: Mode::Forced,
                available: Some(tag),
                pending: None,
            };
        }
        if !published_build() {
            return Self::default();
        }
        let cache = Cache::load();
        let running = crate::version_label();
        Self {
            available: cache.available_for(&running),
            mode: Mode::Live { cache, running },
            pending: None,
        }
    }

    /// Whether this build checks at all, so has a setting to show.
    pub(super) fn applies(&self) -> bool {
        !matches!(self.mode, Mode::Off)
    }

    /// The release to point at, while checks are `enabled`.
    pub(super) fn notice(&self, enabled: bool) -> Option<&Tag> {
        match self.mode {
            Mode::Off => None,
            _ if !enabled => None,
            _ => self.available.as_ref(),
        }
    }

    /// Take an answer that has arrived, and start a check that is due.
    /// Cheap when neither happens: one clock read and a comparison, and not
    /// even that while checks are off.
    pub(super) fn poll(&mut self, ctx: &egui::Context, enabled: bool) {
        let Mode::Live { cache, running } = &mut self.mode else {
            return;
        };
        if let Some(rx) = &self.pending {
            let answer = match rx.try_recv() {
                Err(mpsc::TryRecvError::Empty) => return,
                Ok(answer) => answer,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Err("the check ended without an answer".to_string())
                }
            };
            self.pending = None;
            match answer {
                Ok(found) => {
                    match &found {
                        Some(tag) => log::info!("update check: {} is available", tag.name()),
                        None => log::debug!("update check: up to date"),
                    }
                    cache.available = found.as_ref().map(Tag::name);
                    self.available = found;
                }
                // Nothing to show for it: the user didn't ask. What an
                // earlier answer from this build found still stands.
                Err(e) => {
                    log::debug!("update check failed: {e}");
                    if cache.checked_by.as_deref() != Some(running.as_str()) {
                        cache.available = None;
                    }
                }
            }
            cache.checked_at = Some(now());
            cache.checked_by = Some(running.clone());
            cache.save();
            return;
        }
        if enabled && cache.is_due(now(), running) {
            self.pending = Some(spawn(ctx.clone()));
        }
    }
}

/// Whole seconds, so the cache file reads `2026-09-29T08:15:02Z`.
fn now() -> DateTime<Utc> {
    Utc::now().trunc_subsecs(0)
}

/// Ask GitHub on a thread of its own; the answer arrives on the receiver.
fn spawn(ctx: egui::Context) -> mpsc::Receiver<Answer> {
    let (tx, rx) = mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("update-check".to_string())
        .spawn(move || {
            let answer = std::panic::catch_unwind(fetch)
                .unwrap_or_else(|panic| Err(super::connection::panic_text(&*panic)));
            let _ = tx.send(answer);
            // An idle window doesn't repaint on its own.
            ctx.request_repaint();
        });
    if let Err(e) = spawned {
        log::debug!("update check: could not start a thread: {e}");
    }
    rx
}

fn fetch() -> Answer {
    let version = env!("CARGO_PKG_VERSION");
    let dev = version.contains("-dev");
    // A release only moves to later releases, which `latest` names on its
    // own; it leaves out prereleases, which is every dev build.
    let url = if dev {
        format!("{API}/releases?per_page={PAGE_SIZE}")
    } else {
        format!("{API}/releases/latest")
    };
    let body = get(&url)?;
    let releases: Vec<Release> = if dev {
        serde_json::from_slice(&body)
    } else {
        serde_json::from_slice(&body).map(|r| vec![r])
    }
    .map_err(|e| format!("unexpected response: {e}"))?;
    Ok(newer_release(version, env!("GIT_HASH"), &releases))
}

/// Never called: without the feature no build is published-and-checking.
#[cfg(not(feature = "update-check"))]
fn get(_url: &str) -> Result<Vec<u8>, String> {
    Err("built without the update-check feature".to_string())
}

/// The body GitHub answers `url` with, over HTTPS only.
#[cfg(feature = "update-check")]
fn get(url: &str) -> Result<Vec<u8>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .https_only(true)
        .timeout_global(Some(TIMEOUT))
        .user_agent(format!("dmm-tools/{}", env!("CARGO_PKG_VERSION")))
        // The operating system's trust store, not a bundled CA list; ring as
        // the crypto provider, handed over explicitly because ureq picks none
        // without its bundled list.
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .unversioned_rustls_crypto_provider(std::sync::Arc::new(
                    rustls::crypto::ring::default_provider(),
                ))
                .build(),
        )
        .build()
        .into();
    let mut response = agent
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| e.to_string())?;
    response
        .body_mut()
        .with_config()
        .limit(BODY_LIMIT)
        .read_to_vec()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(rfc3339: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(rfc3339).unwrap().to_utc()
    }

    fn release(tag: &str, published: &str) -> Release {
        Release {
            tag_name: tag.to_string(),
            draft: false,
            published_at: Some(published.to_string()),
        }
    }

    #[test]
    fn tags_parse_only_in_our_two_shapes() {
        assert_eq!(Tag::parse("v0.7.0"), Some(Tag::Release((0, 7, 0))));
        assert_eq!(Tag::parse("v12.34.56"), Some(Tag::Release((12, 34, 56))));
        assert_eq!(Tag::parse("dev-f5ff045"), Some(Tag::Dev("f5ff045".into())));
        let long = format!("dev-{}", "a".repeat(40));
        assert!(Tag::parse(&long).is_some());
        for junk in [
            "",
            "v",
            "0.7.0",
            "v0.7",
            "v0.7.0.1",
            "v0.7.0-dev",
            "v0.7.+1",
            "v0..1",
            "v-1.0.0",
            "v0.7.0 ",
            "dev-",
            "dev-f5ff04",
            "dev-F5FF045",
            "dev-f5ff04g",
            "dev-f5ff045/../x",
            "v99999999999.0.0",
        ] {
            assert_eq!(Tag::parse(junk), None, "{junk:?}");
        }
        assert_eq!(Tag::parse(&format!("dev-{}", "a".repeat(41))), None);
    }

    #[test]
    fn the_link_is_built_from_the_tag() {
        let tag = Tag::parse("v0.8.0").unwrap();
        assert_eq!(
            tag.url(),
            "https://github.com/antoinecellerier/dmm-tools/releases/tag/v0.8.0"
        );
        assert_eq!(tag.label(), "v0.8.0 available \u{2197}");
        assert_eq!(tag.short_label(), "v0.8.0 \u{2197}");
        let dev = Tag::parse("dev-f5ff045").unwrap();
        assert!(dev.url().ends_with("/releases/tag/dev-f5ff045"));
    }

    #[test]
    fn a_release_moves_only_to_a_later_release() {
        let latest = |tag| [release(tag, "2026-09-19T20:17:40Z")];
        assert_eq!(newer_release("0.7.0", "", &latest("v0.7.0")), None);
        assert_eq!(newer_release("0.7.0", "", &latest("v0.6.9")), None);
        assert_eq!(
            newer_release("0.7.0", "", &latest("v0.7.1")),
            Some(Tag::Release((0, 7, 1)))
        );
        assert_eq!(
            newer_release("0.7.0", "", &latest("v1.0.0")),
            Some(Tag::Release((1, 0, 0)))
        );
        // `latest` never names a dev build, but one in the list is ignored.
        assert_eq!(
            newer_release("0.7.0", "abc1234", &latest("dev-f5ff045")),
            None
        );
    }

    #[test]
    fn a_dev_build_moves_to_the_newest_nightly_that_is_not_its_own() {
        let releases = [
            release("dev-f5ff045", "2026-09-28T17:37:40Z"),
            release("dev-e3819f4", "2026-09-27T20:49:15Z"),
            release("v0.7.0", "2026-09-19T20:17:40Z"),
        ];
        assert_eq!(newer_release("0.8.0-dev", "f5ff045", &releases), None);
        assert_eq!(
            newer_release("0.8.0-dev", "e3819f4", &releases),
            Some(Tag::Dev("f5ff045".into()))
        );
        // Pruned from the list: still behind the newest.
        assert_eq!(
            newer_release("0.8.0-dev", "0123abc", &releases),
            Some(Tag::Dev("f5ff045".into()))
        );
        // A longer abbreviation of the same commit is the same build.
        assert_eq!(newer_release("0.8.0-dev", "f5ff0451", &releases), None);
    }

    #[test]
    fn nightlies_are_ordered_by_publication_not_list_order() {
        let releases = [
            release("dev-e3819f4", "2026-09-27T20:49:15Z"),
            release("dev-f5ff045", "2026-09-28T17:37:40Z"),
        ];
        assert_eq!(
            newer_release("0.8.0-dev", "e3819f4", &releases),
            Some(Tag::Dev("f5ff045".into()))
        );
    }

    #[test]
    fn a_release_of_its_version_supersedes_a_dev_build() {
        let releases = [
            release("dev-f5ff045", "2026-10-02T10:00:00Z"),
            release("v0.8.0", "2026-10-01T10:00:00Z"),
        ];
        assert_eq!(
            newer_release("0.8.0-dev", "f5ff045", &releases),
            Some(Tag::Release((0, 8, 0)))
        );
    }

    #[test]
    fn a_patch_to_the_previous_line_does_not_hide_a_newer_nightly() {
        let releases = [
            release("v0.7.1", "2026-09-30T10:00:00Z"),
            release("dev-f5ff045", "2026-09-29T10:00:00Z"),
            release("dev-e3819f4", "2026-09-27T20:49:15Z"),
        ];
        assert_eq!(
            newer_release("0.8.0-dev", "e3819f4", &releases),
            Some(Tag::Dev("f5ff045".into()))
        );
        assert_eq!(newer_release("0.8.0-dev", "f5ff045", &releases), None);
    }

    #[test]
    fn a_build_without_a_commit_only_hears_of_releases() {
        let releases = [release("dev-f5ff045", "2026-09-28T17:37:40Z")];
        assert_eq!(newer_release("0.8.0-dev", "dev", &releases), None);
        assert_eq!(newer_release("0.8.0-dev", "unknown", &releases), None);
        let with_release = [release("v0.8.0", "2026-10-01T10:00:00Z")];
        assert_eq!(
            newer_release("0.8.0-dev", "dev", &with_release),
            Some(Tag::Release((0, 8, 0)))
        );
    }

    #[test]
    fn drafts_junk_and_undated_entries_are_skipped() {
        let mut draft = release("v9.0.0", "2026-10-01T10:00:00Z");
        draft.draft = true;
        let undated = Release {
            tag_name: "v8.0.0".into(),
            draft: false,
            published_at: None,
        };
        let releases = [
            draft,
            undated,
            release("v7.0.0", "yesterday"),
            release("nightly", "2026-10-01T10:00:00Z"),
            release("dev-f5ff045", "2026-09-28T17:37:40Z"),
        ];
        assert_eq!(newer_release("0.8.0-dev", "f5ff045", &releases), None);
        assert_eq!(newer_release("0.7.0", "", &releases), None);
    }

    #[test]
    fn an_unrecognised_own_version_never_claims_an_update() {
        let releases = [release("v9.0.0", "2026-10-01T10:00:00Z")];
        assert_eq!(newer_release("0.8.0-rc1", "f5ff045", &releases), None);
        assert_eq!(newer_release("garbage", "f5ff045", &releases), None);
    }

    /// Trimmed from a real `/releases` response: every other field is ignored.
    #[test]
    fn the_api_response_parses() {
        let body = r#"[{"url":"https://api.github.com/repos/antoinecellerier/dmm-tools/releases/398477742",
            "html_url":"https://github.com/antoinecellerier/dmm-tools/releases/tag/dev-f5ff045",
            "id":398477742,"tag_name":"dev-f5ff045","target_commitish":"main",
            "name":"v0.8.0-dev (f5ff045)","draft":false,"immutable":false,"prerelease":true,
            "created_at":"2026-09-28T17:26:50Z","updated_at":"2026-09-28T17:37:44Z",
            "published_at":"2026-09-28T17:37:40Z","assets":[],"body":"> [!WARNING]\n"}]"#;
        let releases: Vec<Release> = serde_json::from_str(body).unwrap();
        assert_eq!(
            newer_release("0.8.0-dev", "e3819f4", &releases),
            Some(Tag::Dev("f5ff045".into()))
        );
        let latest = r#"{"tag_name":"v0.7.0","draft":false,"prerelease":false,
            "published_at":"2026-09-19T20:17:40Z"}"#;
        let release: Release = serde_json::from_str(latest).unwrap();
        assert_eq!(newer_release("0.7.0", "", &[release]), None);
    }

    #[test]
    fn a_check_is_due_once_a_day_and_after_a_version_change() {
        let running = "v0.8.0-dev (f5ff045)";
        let at = at("2026-09-29T08:15:02Z");
        let cache = Cache {
            checked_at: Some(at),
            checked_by: Some(running.to_string()),
            available: None,
        };
        assert!(Cache::default().is_due(at, running));
        assert!(!cache.is_due(at, running));
        let second = TimeDelta::seconds(1);
        assert!(!cache.is_due(at + INTERVAL - second, running));
        assert!(cache.is_due(at + INTERVAL, running));
        assert!(cache.is_due(at - second, running), "clock set back");
        assert!(cache.is_due(at, "v0.8.0-dev (0123abc)"), "another build");
    }

    #[test]
    fn a_cached_tag_counts_only_for_the_build_that_found_it() {
        let cache = Cache {
            checked_at: Some(at("2026-09-29T08:15:02Z")),
            checked_by: Some("v0.7.0".to_string()),
            available: Some("v0.8.0".to_string()),
        };
        assert_eq!(cache.available_for("v0.7.0"), Some(Tag::Release((0, 8, 0))));
        assert_eq!(cache.available_for("v0.8.0"), None);
        let tampered = Cache {
            available: Some("javascript:alert(1)".to_string()),
            ..cache.clone()
        };
        assert_eq!(tampered.available_for("v0.7.0"), None);
    }

    #[test]
    fn the_cache_round_trips_and_tolerates_missing_fields() {
        let cache = Cache {
            checked_at: Some(at("2026-09-29T08:15:02Z")),
            checked_by: Some("v0.7.0".to_string()),
            available: Some("v0.8.0".to_string()),
        };
        let json = serde_json::to_string(&cache).unwrap();
        assert!(
            json.contains(r#""checked_at":"2026-09-29T08:15:02Z""#),
            "{json}"
        );
        assert_eq!(serde_json::from_str::<Cache>(&json).unwrap(), cache);
        assert_eq!(
            serde_json::from_str::<Cache>("{}").unwrap(),
            Cache::default()
        );
    }

    /// The one test that reaches GitHub, so ignored: run it by hand after
    /// touching the request, `cargo test -p dmm-gui -- --ignored the_real_api`.
    #[test]
    #[cfg(feature = "update-check")]
    #[ignore = "reaches api.github.com"]
    fn the_real_api_answers() {
        let answer = fetch();
        assert!(answer.is_ok(), "{answer:?}");
    }

    #[test]
    fn a_source_build_neither_shows_nor_asks() {
        let check = UpdateCheck::default();
        assert!(!check.applies());
        assert_eq!(check.notice(true), None);
    }

    #[test]
    fn the_hidden_flag_shows_its_tag_while_the_setting_is_on() {
        let tag = Tag::parse("v0.8.0").unwrap();
        let check = UpdateCheck::new(Some(tag.clone()));
        assert!(check.applies());
        assert_eq!(check.notice(true), Some(&tag));
        assert_eq!(check.notice(false), None);
    }
}
