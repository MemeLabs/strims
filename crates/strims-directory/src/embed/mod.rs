// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Embed listings: the services a directory takes them for, their ids,
//! and the loaders of their snippets from the services' APIs.
//!
//! A directory takes an embed listing of a service its server config
//! enables with the service's credentials ([`LoaderConfig`]): AngelThump
//! needs none, Twitch a client id and secret (for streams and VODs alike),
//! YouTube a public API key. An id is checked against its service's form
//! ([`validate_id`]) before anything is built from it.
//!
//! # Loads
//!
//! A [`Loader`] turns a load's ids into snippets. Each service's ids go in
//! batches of [`EmbedService::batch_size`], each batch one or two requests
//! made one after the other, and at most [`MAX_IN_FLIGHT`] batches run at
//! once. Every request races a deadline of [`REQUEST_TIMEOUT`] on the
//! caller's timer, and takes a response body of at most
//! [`MAX_RESPONSE_BYTES`]. A batch that fails yields no snippets; the
//! caller keeps the snippets it had.
//!
//! Twitch requests carry an app access token, fetched once per load when
//! there is none, kept until a minute before it expires, and dropped when
//! Twitch answers 401.
//!
//! Each service's loader is a pure request builder and a parser. Parsers
//! take a field of an unexpected type as its default, so one odd field
//! doesn't lose the batch. A snippet's text fields are clamped to
//! [`MAX_TITLE_BYTES`] and [`MAX_NAME_BYTES`], and an image keeps only an
//! `https://` URL of at most [`MAX_URL_BYTES`].

mod angelthump;
pub mod rfc3339;
mod twitch;
mod youtube;

use std::cell::RefCell;
use std::collections::BTreeMap;

use futures_util::future::{Either, select};
use futures_util::stream::{self, StreamExt};
use serde::Deserialize;
use serde::de::{DeserializeOwned, Deserializer};
use serde_json::Value;
use strims_base::time::{Duration, Time};
use strims_proto::network::v1::directory::listing::embed::Service;
use strims_proto::network::v1::directory::listing::{Content, Embed};
use strims_proto::network::v1::directory::listing_snippet_image::SourceOneof;
use strims_proto::network::v1::directory::{
    Listing, ListingSnippet, ListingSnippetImage, ServerConfig,
};
use strims_rpc::Timer;

use crate::DirectoryError;
use crate::fetch::{Fetch, FetchError, HttpRequest};

/// How long one request may take.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// The most batches, and so requests, a load has in flight.
pub const MAX_IN_FLIGHT: usize = 4;
/// The longest response body a loader takes.
pub const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
/// The longest title a loaded snippet keeps.
pub const MAX_TITLE_BYTES: usize = 1024;
/// The longest channel name a loaded snippet keeps.
pub const MAX_NAME_BYTES: usize = 256;
/// The longest image URL a loaded snippet keeps.
pub const MAX_URL_BYTES: usize = 2048;

/// An embed listing's service.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EmbedService {
    AngelThump,
    TwitchStream,
    TwitchVod,
    YouTube,
}

impl EmbedService {
    pub const ALL: [EmbedService; 4] = [
        EmbedService::AngelThump,
        EmbedService::TwitchStream,
        EmbedService::TwitchVod,
        EmbedService::YouTube,
    ];

    /// The service of an `Embed.service` value; none for an unknown one or
    /// `UNDEFINED`.
    pub fn from_proto(v: i32) -> Option<EmbedService> {
        match Service::try_from(v).ok()? {
            Service::DirectoryListingEmbedServiceUndefined => None,
            Service::DirectoryListingEmbedServiceAngelthump => Some(EmbedService::AngelThump),
            Service::DirectoryListingEmbedServiceTwitchStream => Some(EmbedService::TwitchStream),
            Service::DirectoryListingEmbedServiceTwitchVod => Some(EmbedService::TwitchVod),
            Service::DirectoryListingEmbedServiceYoutube => Some(EmbedService::YouTube),
        }
    }

    /// The service's `Embed.service` value.
    pub fn as_proto(self) -> i32 {
        let s = match self {
            EmbedService::AngelThump => Service::DirectoryListingEmbedServiceAngelthump,
            EmbedService::TwitchStream => Service::DirectoryListingEmbedServiceTwitchStream,
            EmbedService::TwitchVod => Service::DirectoryListingEmbedServiceTwitchVod,
            EmbedService::YouTube => Service::DirectoryListingEmbedServiceYoutube,
        };
        s as i32
    }

    /// The most ids one batch of the service's API takes.
    pub fn batch_size(self) -> usize {
        match self {
            EmbedService::AngelThump => 1,
            EmbedService::TwitchStream | EmbedService::TwitchVod => twitch::MAX_RESULTS,
            EmbedService::YouTube => youtube::MAX_RESULTS,
        }
    }

    fn is_twitch(self) -> bool {
        matches!(self, EmbedService::TwitchStream | EmbedService::TwitchVod)
    }
}

/// The embed services a directory takes listings of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EmbedServices {
    pub angelthump: bool,
    /// Twitch streams and VODs.
    pub twitch: bool,
    pub youtube: bool,
}

impl EmbedServices {
    pub fn supports(self, s: EmbedService) -> bool {
        match s {
            EmbedService::AngelThump => self.angelthump,
            EmbedService::TwitchStream | EmbedService::TwitchVod => self.twitch,
            EmbedService::YouTube => self.youtube,
        }
    }

    /// Whether the set has any service.
    pub fn any(self) -> bool {
        self.angelthump || self.twitch || self.youtube
    }
}

/// Checks `id` against `service`'s form, so that a request built from it
/// names one channel or video ([`DirectoryError::InvalidEmbedId`]):
/// - a Twitch stream: a login of 1 to 25 characters of `[A-Za-z0-9_]`;
/// - a Twitch VOD: 1 to 20 digits;
/// - a YouTube video: 11 characters of `[A-Za-z0-9_-]`;
/// - an AngelThump stream: a username of 1 to 32 characters of
///   `[A-Za-z0-9_-]`.
pub fn validate_id(service: EmbedService, id: &str) -> Result<(), DirectoryError> {
    let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let ok = match service {
        EmbedService::TwitchStream => (1..=25).contains(&id.len()) && id.bytes().all(word),
        EmbedService::TwitchVod => {
            (1..=20).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_digit())
        }
        EmbedService::YouTube => id.len() == 11 && id.bytes().all(|b| word(b) || b == b'-'),
        EmbedService::AngelThump => {
            (1..=32).contains(&id.len()) && id.bytes().all(|b| word(b) || b == b'-')
        }
    };
    if ok {
        Ok(())
    } else {
        Err(DirectoryError::InvalidEmbedId)
    }
}

/// The embed listing of `service`'s `id`.
pub fn listing(service: EmbedService, id: &str) -> Listing {
    Listing {
        content: Some(Content::Embed(Embed {
            service: service.as_proto(),
            id: id.to_owned(),
            query_params: BTreeMap::new(),
        })),
    }
}

/// The service and id of an embed listing of a known service.
pub fn of(l: &Listing) -> Option<(EmbedService, &str)> {
    match &l.content {
        Some(Content::Embed(e)) => Some((EmbedService::from_proto(e.service)?, e.id.as_str())),
        _ => None,
    }
}

/// The Twitch app credentials.
#[derive(Clone, PartialEq, Eq)]
pub struct TwitchCredentials {
    pub client_id: String,
    pub client_secret: String,
}

impl std::fmt::Debug for TwitchCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TwitchCredentials")
            .field("client_id", &self.client_id)
            .finish_non_exhaustive()
    }
}

/// The embed services a server config enables with their credentials.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoaderConfig {
    pub angelthump: bool,
    pub twitch: Option<TwitchCredentials>,
    /// YouTube's public API key.
    pub youtube: Option<String>,
}

impl LoaderConfig {
    /// The services `c` enables: AngelThump if enabled, Twitch and
    /// YouTube if enabled with non-empty credentials.
    pub fn from_config(c: Option<&ServerConfig>) -> LoaderConfig {
        let i = c.and_then(|c| c.integrations.as_ref());
        LoaderConfig {
            angelthump: i
                .and_then(|i| i.angelthump.as_ref())
                .is_some_and(|a| a.enable),
            twitch: i
                .and_then(|i| i.twitch.as_ref())
                .filter(|t| t.enable && !t.client_id.is_empty() && !t.client_secret.is_empty())
                .map(|t| TwitchCredentials {
                    client_id: t.client_id.clone(),
                    client_secret: t.client_secret.clone(),
                }),
            youtube: i
                .and_then(|i| i.youtube.as_ref())
                .filter(|y| y.enable && !y.public_api_key.is_empty())
                .map(|y| y.public_api_key.clone()),
        }
    }

    pub fn services(&self) -> EmbedServices {
        EmbedServices {
            angelthump: self.angelthump,
            twitch: self.twitch.is_some(),
            youtube: self.youtube.is_some(),
        }
    }
}

/// Why a batch failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LoadError {
    #[error(transparent)]
    Fetch(FetchError),
    #[error("request timed out")]
    Timeout,
    #[error("http status {0}")]
    Status(u16),
    #[error("unexpected response: {0}")]
    Parse(String),
    /// The loader's config doesn't enable the service.
    #[error("service not enabled")]
    Disabled,
}

fn parse_error(e: serde_json::Error) -> LoadError {
    LoadError::Parse(e.to_string())
}

/// The snippets of one batch: each id the batch named that the service
/// knows, with its snippet.
pub type Snippets = Vec<(String, ListingSnippet)>;

/// Loads embed snippets for a directory; see the [module docs](self).
#[derive(Debug, Default)]
pub struct Loader {
    config: LoaderConfig,
    /// Twitch's app token. Borrowed only between awaits.
    token: RefCell<Option<twitch::Token>>,
}

impl Loader {
    pub fn new(config: LoaderConfig) -> Loader {
        Loader {
            config,
            token: RefCell::new(None),
        }
    }

    pub fn config(&self) -> &LoaderConfig {
        &self.config
    }

    /// Loads the snippets of `ids` through `fetch`, timed on `timer`, and
    /// hands `done` each batch's outcome as it ends.
    pub async fn load<T: Timer>(
        &self,
        fetch: &Fetch,
        timer: &T,
        ids: BTreeMap<EmbedService, Vec<String>>,
        mut done: impl FnMut(EmbedService, Result<Snippets, LoadError>),
    ) {
        let token = match &self.config.twitch {
            Some(c) if ids.keys().any(|s| s.is_twitch()) => {
                Some(self.twitch_token(fetch, timer, c).await)
            }
            _ => None,
        };
        let token = token.as_ref();
        let batches: Vec<(EmbedService, Vec<String>)> = ids
            .into_iter()
            .flat_map(|(s, ids)| {
                ids.chunks(s.batch_size())
                    .map(|c| (s, c.to_vec()))
                    .collect::<Vec<_>>()
            })
            .collect();
        let mut runs = stream::iter(batches)
            .map(|(s, ids)| async move { (s, self.batch(fetch, timer, token, s, &ids).await) })
            .buffer_unordered(MAX_IN_FLIGHT);
        while let Some((s, res)) = runs.next().await {
            done(s, res);
        }
    }

    async fn batch<T: Timer>(
        &self,
        fetch: &Fetch,
        timer: &T,
        token: Option<&Result<String, LoadError>>,
        service: EmbedService,
        ids: &[String],
    ) -> Result<Snippets, LoadError> {
        match service {
            EmbedService::AngelThump => {
                let Some(id) = ids.first().filter(|_| self.config.angelthump) else {
                    return Err(LoadError::Disabled);
                };
                let body = send(fetch, timer, angelthump::request(id)).await?;
                let streams = angelthump::parse(&body)?;
                Ok(angelthump::snippets(ids, &streams, timer.now()))
            }
            EmbedService::TwitchStream | EmbedService::TwitchVod => {
                let (Some(c), Some(token)) = (&self.config.twitch, token) else {
                    return Err(LoadError::Disabled);
                };
                let token = token.clone()?;
                if service == EmbedService::TwitchStream {
                    let body = self
                        .helix(fetch, timer, twitch::users_request(c, &token, ids))
                        .await?;
                    let users = twitch::parse_users(&body)?;
                    let body = self
                        .helix(fetch, timer, twitch::streams_request(c, &token, ids))
                        .await?;
                    let streams = twitch::parse_streams(&body)?;
                    Ok(twitch::stream_snippets(ids, &users, &streams, timer.now()))
                } else {
                    let body = self
                        .helix(fetch, timer, twitch::videos_request(c, &token, ids))
                        .await?;
                    let videos = twitch::parse_videos(&body)?;
                    let logins = twitch::logins(ids, &videos);
                    let users = if logins.is_empty() {
                        Vec::new()
                    } else {
                        let req = twitch::users_request(c, &token, &logins);
                        twitch::parse_users(&self.helix(fetch, timer, req).await?)?
                    };
                    Ok(twitch::vod_snippets(ids, &videos, &users))
                }
            }
            EmbedService::YouTube => {
                let Some(key) = self.config.youtube.as_deref() else {
                    return Err(LoadError::Disabled);
                };
                let body = send(fetch, timer, youtube::videos_request(key, ids)).await?;
                let videos = youtube::parse_videos(&body)?;
                let channel_ids = youtube::channel_ids(ids, &videos);
                let channels = if channel_ids.is_empty() {
                    Vec::new()
                } else {
                    let req = youtube::channels_request(key, &channel_ids);
                    youtube::parse_channels(&send(fetch, timer, req).await?)?
                };
                Ok(youtube::snippets(ids, &videos, &channels))
            }
        }
    }

    /// A Helix request, dropping the token when Twitch refuses it.
    async fn helix<T: Timer>(
        &self,
        fetch: &Fetch,
        timer: &T,
        req: HttpRequest,
    ) -> Result<Vec<u8>, LoadError> {
        let res = send(fetch, timer, req).await;
        if matches!(res, Err(LoadError::Status(401))) {
            let dropped = self.token.borrow_mut().take();
            drop(dropped);
        }
        res
    }

    /// The cached Twitch token, or a new one.
    async fn twitch_token<T: Timer>(
        &self,
        fetch: &Fetch,
        timer: &T,
        c: &TwitchCredentials,
    ) -> Result<String, LoadError> {
        let now = timer.now();
        let cached = self
            .token
            .borrow()
            .as_ref()
            .filter(|t| now < t.expires)
            .map(|t| t.access_token.clone());
        if let Some(t) = cached {
            return Ok(t);
        }
        let body = send(fetch, timer, twitch::token_request(c)).await?;
        let token = twitch::parse_token(&body, now)?;
        let access = token.access_token.clone();
        let kept = (token.expires > now).then_some(token);
        let prev = self.token.replace(kept);
        drop(prev);
        Ok(access)
    }
}

/// Sends `req` through `fetch` within [`REQUEST_TIMEOUT`]: the body of a
/// 2xx response.
async fn send<T: Timer>(fetch: &Fetch, timer: &T, req: HttpRequest) -> Result<Vec<u8>, LoadError> {
    req.check().map_err(LoadError::Fetch)?;
    let max = req.max_response_bytes;
    let deadline = Box::pin(timer.sleep_until(timer.now() + REQUEST_TIMEOUT));
    let res = match select(fetch.fetch(req), deadline).await {
        Either::Left((res, _)) => res.map_err(LoadError::Fetch)?,
        Either::Right(_) => return Err(LoadError::Timeout),
    };
    if !(200..300).contains(&res.status) {
        return Err(LoadError::Status(res.status));
    }
    if res.body.len() > max {
        return Err(LoadError::Fetch(FetchError::TooLarge));
    }
    Ok(res.body)
}

// --- snippet fields ---------------------------------------------------------

/// `s`, cut to at most `max` bytes at a character boundary.
fn clamp(s: &str, max: usize) -> String {
    let mut end = s.len().min(max);
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s.get(..end).unwrap_or_default().to_owned()
}

/// The image at `url`, if it's an `https://` URL of at most
/// [`MAX_URL_BYTES`] without control characters.
fn url_image(url: &str) -> Option<ListingSnippetImage> {
    let ok = url.starts_with("https://")
        && url.len() <= MAX_URL_BYTES
        && !url.chars().any(char::is_control);
    ok.then(|| ListingSnippetImage {
        source_oneof: Some(SourceOneof::Url(url.to_owned())),
    })
}

/// `url` with `_t` set to `now` in hex seconds, so that viewers fetch a
/// live thumbnail again at every load.
fn cache_busted(url: &str, now: Time) -> String {
    if url.is_empty() {
        return String::new();
    }
    let sep = if url.contains('?') { '&' } else { '?' };
    format!("{url}{sep}_t={:x}", now.unix())
}

// --- lenient JSON -----------------------------------------------------------

/// A JSON string; empty for any other value.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Text(String);

impl<'de> Deserialize<'de> for Text {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Text, D::Error> {
        Ok(Text(match Value::deserialize(d)? {
            Value::String(s) => s,
            _ => String::new(),
        }))
    }
}

impl Text {
    fn as_str(&self) -> &str {
        &self.0
    }
}

/// A JSON count: a number or a string of digits, 0 for anything else or
/// below 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Count(u64);

impl<'de> Deserialize<'de> for Count {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Count, D::Error> {
        Ok(Count(match Value::deserialize(d)? {
            Value::Number(n) => n
                .as_u64()
                .or_else(|| n.as_f64().filter(|f| *f > 0.0).map(|f| f as u64))
                .unwrap_or(0),
            Value::String(s) => s.trim().parse().unwrap_or(0),
            _ => 0,
        }))
    }
}

/// A JSON boolean; false for any other value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Flag(bool);

impl<'de> Deserialize<'de> for Flag {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Flag, D::Error> {
        Ok(Flag(matches!(Value::deserialize(d)?, Value::Bool(true))))
    }
}

/// The elements of a JSON array that decode as `T`; empty for any other
/// value.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Items<T>(Vec<T>);

impl<T> Default for Items<T> {
    fn default() -> Items<T> {
        Items(Vec::new())
    }
}

impl<'de, T: DeserializeOwned> Deserialize<'de> for Items<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Items<T>, D::Error> {
        Ok(Items(match Value::deserialize(d)? {
            Value::Array(a) => a
                .into_iter()
                .filter_map(|v| serde_json::from_value(v).ok())
                .collect(),
            _ => Vec::new(),
        }))
    }
}

/// A JSON value as `T`, or `T`'s default if it doesn't decode.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Or<T>(T);

impl<'de, T: DeserializeOwned + Default> Deserialize<'de> for Or<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Or<T>, D::Error> {
        Ok(Or(
            serde_json::from_value(Value::deserialize(d)?).unwrap_or_default()
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )]

    use std::cell::RefCell;
    use std::rc::Rc;

    use futures_util::future::LocalBoxFuture;
    use strims_proto::network::v1::directory::server_config::Integrations;
    use strims_proto::network::v1::directory::server_config::integrations::{
        AngelThump, Twitch, YouTube,
    };

    use super::*;
    use crate::fetch::{HttpFetch, HttpResponse, Method};

    #[test]
    fn ids_are_checked_by_service() {
        use EmbedService::*;
        let ok = [
            (TwitchStream, "a"),
            (TwitchStream, "Destiny_1"),
            (TwitchStream, &*"a".repeat(25)),
            (TwitchVod, "1157956746"),
            (TwitchVod, &*"9".repeat(20)),
            (YouTube, "-IO6fpjDJY8"),
            (YouTube, "JQ4Jx8XfP_w"),
            (AngelThump, "psrngafk"),
            (AngelThump, "a-b_C9"),
            (AngelThump, &*"a".repeat(32)),
        ];
        for (s, id) in ok {
            assert_eq!(validate_id(s, id), Ok(()), "{s:?} {id}");
        }
        let bad = [
            (TwitchStream, ""),
            (TwitchStream, &*"a".repeat(26)),
            (TwitchStream, "a-b"),
            (TwitchStream, "a&login=b"),
            (TwitchVod, "12a"),
            (TwitchVod, &*"9".repeat(21)),
            (TwitchVod, ""),
            (YouTube, "-IO6fpjDJY"),
            (YouTube, "-IO6fpjDJY88"),
            (YouTube, "-IO6fpjDJY,"),
            (AngelThump, ""),
            (AngelThump, &*"a".repeat(33)),
            (AngelThump, "a b"),
            (AngelThump, "a/../b"),
            (AngelThump, "é"),
        ];
        for (s, id) in bad {
            assert_eq!(
                validate_id(s, id),
                Err(DirectoryError::InvalidEmbedId),
                "{s:?} {id}"
            );
        }
    }

    #[test]
    fn services_round_trip_their_proto_values() {
        for s in EmbedService::ALL {
            assert_eq!(EmbedService::from_proto(s.as_proto()), Some(s));
            assert_eq!(of(&listing(s, "x")), Some((s, "x")));
        }
        assert_eq!(EmbedService::from_proto(0), None);
        assert_eq!(EmbedService::from_proto(5), None);
        assert_eq!(EmbedService::from_proto(-1), None);
        assert_eq!(EmbedService::AngelThump.as_proto(), 1);
        assert_eq!(EmbedService::YouTube.as_proto(), 4);
    }

    fn integrations(f: impl FnOnce(&mut Integrations)) -> ServerConfig {
        let mut i = Integrations::default();
        f(&mut i);
        ServerConfig {
            integrations: Some(i),
            ..Default::default()
        }
    }

    #[test]
    fn services_need_their_credentials() {
        assert_eq!(LoaderConfig::from_config(None), LoaderConfig::default());
        let all = integrations(|i| {
            i.angelthump = Some(AngelThump { enable: true });
            i.twitch = Some(Twitch {
                enable: true,
                client_id: "id".into(),
                client_secret: "secret".into(),
            });
            i.youtube = Some(YouTube {
                enable: true,
                public_api_key: "key".into(),
            });
        });
        let c = LoaderConfig::from_config(Some(&all));
        assert_eq!(
            c.services(),
            EmbedServices {
                angelthump: true,
                twitch: true,
                youtube: true
            }
        );
        assert_eq!(c.youtube.as_deref(), Some("key"));
        assert!(!format!("{c:?}").contains("secret"));

        let missing = integrations(|i| {
            i.twitch = Some(Twitch {
                enable: true,
                client_id: "id".into(),
                client_secret: String::new(),
            });
            i.youtube = Some(YouTube {
                enable: true,
                public_api_key: String::new(),
            });
            i.angelthump = Some(AngelThump { enable: false });
        });
        assert!(!LoaderConfig::from_config(Some(&missing)).services().any());
        let disabled = integrations(|i| {
            i.youtube = Some(YouTube {
                enable: false,
                public_api_key: "key".into(),
            });
        });
        assert!(!LoaderConfig::from_config(Some(&disabled)).services().any());
        let s = EmbedServices {
            twitch: true,
            ..Default::default()
        };
        assert!(s.supports(EmbedService::TwitchStream) && s.supports(EmbedService::TwitchVod));
        assert!(!s.supports(EmbedService::YouTube) && !s.supports(EmbedService::AngelThump));
    }

    #[test]
    fn snippet_fields_are_bounded() {
        assert_eq!(clamp("héllo", 2), "h");
        assert_eq!(clamp("héllo", 3), "hé");
        assert_eq!(clamp("abc", 10), "abc");
        assert!(url_image("https://x/a.png").is_some());
        for bad in [
            "",
            "http://x/a.png",
            "javascript:alert(1)",
            "https://x/\n",
            &format!("https://x/{}", "a".repeat(MAX_URL_BYTES)),
        ] {
            assert_eq!(url_image(bad), None, "{bad}");
        }
        let now = Time::from_unix(0x6553_f100, 0);
        assert_eq!(
            cache_busted("https://x/a.jpg", now),
            "https://x/a.jpg?_t=6553f100"
        );
        assert_eq!(
            cache_busted("https://x/a?b=c", now),
            "https://x/a?b=c&_t=6553f100"
        );
        assert_eq!(cache_busted("", now), "");
    }

    #[derive(Deserialize, Default, Debug, PartialEq)]
    #[serde(default)]
    struct Probe {
        text: Text,
        count: Count,
        flag: Flag,
        items: Items<Text>,
        inner: Or<Inner>,
    }

    #[derive(Deserialize, Default, Debug, PartialEq)]
    #[serde(default)]
    struct Inner {
        a: Text,
    }

    #[test]
    fn odd_fields_take_their_defaults() {
        let p: Probe = serde_json::from_str(
            r#"{"text":"t","count":"12","flag":true,"items":["a",1,"b"],"inner":{"a":"x"},"other":[1]}"#,
        )
        .unwrap();
        assert_eq!(p.text.as_str(), "t");
        assert_eq!(p.count.0, 12);
        assert!(p.flag.0);
        assert_eq!(
            p.items.0,
            [Text("a".into()), Text(String::new()), Text("b".into())]
        );
        assert_eq!(p.inner.0.a.as_str(), "x");

        let p: Probe = serde_json::from_str(
            r#"{"text":1,"count":-3,"flag":"yes","items":{"a":1},"inner":[1]}"#,
        )
        .unwrap();
        assert_eq!(p, Probe::default());
        let p: Probe = serde_json::from_str(r#"{"count":2.7,"text":null}"#).unwrap();
        assert_eq!(p.count.0, 2);
        let p: Probe = serde_json::from_str(r#"{"count":18446744073709551615}"#).unwrap();
        assert_eq!(p.count.0, u64::MAX);
        assert!(serde_json::from_str::<Probe>("[").is_err());
    }

    /// A fetcher answering from a table of (url prefix, status, body), and
    /// recording the requests.
    #[derive(Clone, Default)]
    pub(crate) struct Canned {
        pub(crate) routes: Rc<RefCell<Vec<(String, u16, String)>>>,
        pub(crate) requests: Rc<RefCell<Vec<HttpRequest>>>,
    }

    impl Canned {
        pub(crate) fn route(&self, prefix: &str, status: u16, body: &str) {
            self.routes
                .borrow_mut()
                .push((prefix.into(), status, body.into()));
        }

        fn urls(&self) -> Vec<String> {
            self.requests
                .borrow()
                .iter()
                .map(|r| r.url.clone())
                .collect()
        }
    }

    impl HttpFetch for Canned {
        fn fetch(
            &self,
            req: HttpRequest,
        ) -> LocalBoxFuture<'static, Result<HttpResponse, FetchError>> {
            let route = self
                .routes
                .borrow()
                .iter()
                .find(|(p, _, _)| req.url.starts_with(p.as_str()))
                .cloned();
            self.requests.borrow_mut().push(req);
            Box::pin(async move {
                let (_, status, body) = route.ok_or(FetchError::Connect("no route".into()))?;
                Ok(HttpResponse {
                    status,
                    headers: Vec::new(),
                    body: body.into_bytes(),
                })
            })
        }
    }

    /// A timer whose clock stands still at `now`, and whose sleeps never
    /// end unless `expired`.
    #[derive(Clone)]
    struct Still {
        now: Time,
        expired: bool,
    }

    impl Timer for Still {
        type Sleep = LocalBoxFuture<'static, ()>;

        fn now(&self) -> Time {
            self.now
        }

        fn sleep_until(&self, _: Time) -> Self::Sleep {
            if self.expired {
                Box::pin(std::future::ready(()))
            } else {
                Box::pin(std::future::pending())
            }
        }
    }

    const NOW: Time = Time::from_unix(1_700_000_000, 0);

    fn still() -> Still {
        Still {
            now: NOW,
            expired: false,
        }
    }

    fn run_load(
        loader: &Loader,
        fetch: &Canned,
        timer: &Still,
        ids: &[(EmbedService, &[&str])],
    ) -> Vec<(EmbedService, Result<Snippets, LoadError>)> {
        let ids = ids
            .iter()
            .map(|(s, ids)| (*s, ids.iter().map(|i| (*i).to_owned()).collect()))
            .collect();
        let mut out = Vec::new();
        let f = Fetch::new(fetch.clone());
        futures_util::FutureExt::now_or_never(loader.load(&f, timer, ids, |s, r| out.push((s, r))))
            .expect("the load waits on nothing");
        out
    }

    fn twitch_loader() -> Loader {
        Loader::new(LoaderConfig {
            twitch: Some(TwitchCredentials {
                client_id: "cid".into(),
                client_secret: "sec ret".into(),
            }),
            ..Default::default()
        })
    }

    const TOKEN: &str = r#"{"access_token":"tok","expires_in":3600,"token_type":"bearer"}"#;

    #[test]
    fn twitch_takes_one_token_for_many_loads_until_it_expires() {
        let fetch = Canned::default();
        fetch.route("https://id.twitch.tv/oauth2/token", 200, TOKEN);
        fetch.route("https://api.twitch.tv/helix/", 200, r#"{"data":[]}"#);
        let loader = twitch_loader();
        let ids: &[(EmbedService, &[&str])] = &[(EmbedService::TwitchStream, &["a"])];
        let out = run_load(&loader, &fetch, &still(), ids);
        assert_eq!(out, [(EmbedService::TwitchStream, Ok(Vec::new()))]);
        let token = fetch.requests.borrow()[0].clone();
        assert_eq!(token.method, Method::Post);
        assert_eq!(
            token.body,
            b"client_id=cid&client_secret=sec%20ret&grant_type=client_credentials"
        );
        assert!(!token.url.contains("sec"));
        let helix = fetch.requests.borrow()[1].clone();
        assert!(
            helix
                .headers
                .contains(&("Authorization".into(), "Bearer tok".into()))
        );
        assert!(helix.headers.contains(&("Client-Id".into(), "cid".into())));

        run_load(&loader, &fetch, &still(), ids);
        let tokens = |f: &Canned| f.urls().iter().filter(|u| u.contains("oauth2")).count();
        assert_eq!(tokens(&fetch), 1);
        let later = Still {
            now: NOW + Duration::from_secs(3600 - 60),
            expired: false,
        };
        run_load(&loader, &fetch, &later, ids);
        assert_eq!(tokens(&fetch), 2);
    }

    #[test]
    fn a_refused_twitch_token_is_fetched_again() {
        let fetch = Canned::default();
        fetch.route("https://id.twitch.tv/oauth2/token", 200, TOKEN);
        fetch.route("https://api.twitch.tv/helix/", 401, "{}");
        let loader = twitch_loader();
        let ids: &[(EmbedService, &[&str])] = &[(EmbedService::TwitchVod, &["1"])];
        let out = run_load(&loader, &fetch, &still(), ids);
        assert_eq!(
            out,
            [(EmbedService::TwitchVod, Err(LoadError::Status(401)))]
        );
        run_load(&loader, &fetch, &still(), ids);
        let tokens = fetch.urls().iter().filter(|u| u.contains("oauth2")).count();
        assert_eq!(tokens, 2);
    }

    #[test]
    fn a_failed_token_fails_every_twitch_batch() {
        let fetch = Canned::default();
        fetch.route("https://id.twitch.tv/oauth2/token", 500, "");
        let loader = twitch_loader();
        let many: Vec<String> = (0..150).map(|i| format!("u{i}")).collect();
        let many: Vec<&str> = many.iter().map(String::as_str).collect();
        let out = run_load(
            &loader,
            &fetch,
            &still(),
            &[(EmbedService::TwitchStream, &many)],
        );
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|(_, r)| *r == Err(LoadError::Status(500))));
        assert_eq!(fetch.requests.borrow().len(), 1);
    }

    #[test]
    fn ids_go_in_batches_of_their_service() {
        let fetch = Canned::default();
        fetch.route("https://www.googleapis.com/", 200, r#"{"items":[]}"#);
        fetch.route("https://api.angelthump.com/", 200, "[]");
        let loader = Loader::new(LoaderConfig {
            angelthump: true,
            youtube: Some("k".into()),
            ..Default::default()
        });
        let yt: Vec<String> = (0..120).map(|i| format!("{i:011}")).collect();
        let yt: Vec<&str> = yt.iter().map(String::as_str).collect();
        let out = run_load(
            &loader,
            &fetch,
            &still(),
            &[
                (EmbedService::AngelThump, &["a", "b", "c"]),
                (EmbedService::YouTube, &yt),
            ],
        );
        assert_eq!(out.len(), 6);
        assert!(out.iter().all(|(_, r)| r.is_ok()));
        let urls = fetch.urls();
        assert_eq!(urls.iter().filter(|u| u.contains("angelthump")).count(), 3);
        // each batch's ids are joined by commas, after the 4 of `part`
        let mut commas: Vec<usize> = urls
            .iter()
            .filter(|u| u.contains("/videos?"))
            .map(|u| u.matches("%2C").count())
            .collect();
        commas.sort_unstable();
        assert_eq!(commas, [19 + 4, 49 + 4, 49 + 4]);
        let twitch = run_load(
            &loader,
            &fetch,
            &still(),
            &[(EmbedService::TwitchStream, &["a"])],
        );
        assert_eq!(
            twitch,
            [(EmbedService::TwitchStream, Err(LoadError::Disabled))]
        );
    }

    #[test]
    fn a_request_past_its_deadline_fails_the_batch() {
        #[derive(Clone)]
        struct Never;
        impl HttpFetch for Never {
            fn fetch(
                &self,
                _: HttpRequest,
            ) -> LocalBoxFuture<'static, Result<HttpResponse, FetchError>> {
                Box::pin(std::future::pending())
            }
        }
        let loader = Loader::new(LoaderConfig {
            angelthump: true,
            ..Default::default()
        });
        let timer = Still {
            now: NOW,
            expired: true,
        };
        let mut out = Vec::new();
        let ids = BTreeMap::from([(EmbedService::AngelThump, vec!["a".to_owned()])]);
        futures_util::FutureExt::now_or_never(loader.load(
            &Fetch::new(Never),
            &timer,
            ids,
            |s, r| {
                out.push((s, r));
            },
        ))
        .unwrap();
        assert_eq!(out, [(EmbedService::AngelThump, Err(LoadError::Timeout))]);
    }

    #[test]
    fn a_response_over_the_cap_or_with_an_error_status_fails_the_batch() {
        let fetch = Canned::default();
        let big = format!("[\"{}\"]", "a".repeat(MAX_RESPONSE_BYTES));
        fetch.route(
            "https://api.angelthump.com/v3/streams/?username=big",
            200,
            &big,
        );
        fetch.route(
            "https://api.angelthump.com/v3/streams/?username=gone",
            404,
            "",
        );
        fetch.route(
            "https://api.angelthump.com/v3/streams/?username=junk",
            200,
            "<html>",
        );
        let loader = Loader::new(LoaderConfig {
            angelthump: true,
            ..Default::default()
        });
        let out = run_load(
            &loader,
            &fetch,
            &still(),
            &[(EmbedService::AngelThump, &["big", "gone", "junk"])],
        );
        let mut errors: Vec<String> = out
            .iter()
            .map(|(_, r)| match r {
                Err(LoadError::Parse(_)) => "parse".to_owned(),
                r => format!("{r:?}"),
            })
            .collect();
        errors.sort();
        assert_eq!(
            errors,
            [
                "Err(Fetch(TooLarge))".to_owned(),
                "Err(Status(404))".to_owned(),
                "parse".to_owned()
            ]
        );
    }
}
