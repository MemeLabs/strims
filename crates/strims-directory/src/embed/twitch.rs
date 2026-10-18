// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Twitch streams and VODs, from the Helix API with an app access token.
//!
//! Logins are case-insensitive, so a listing's id matches the user whose
//! login equals it in any case.

use std::collections::BTreeMap;

use serde::Deserialize;
use strims_base::time::{Duration, Time};
use strims_proto::network::v1::directory::ListingSnippet;

use super::{
    Count, Flag, Items, LoadError, MAX_NAME_BYTES, MAX_RESPONSE_BYTES, MAX_TITLE_BYTES, Snippets,
    Text, TwitchCredentials, cache_busted, clamp, parse_error, rfc3339, url_image,
};
use crate::fetch::{HttpRequest, Method, query};

/// The most ids a Helix request takes, and the page size it asks for.
pub(super) const MAX_RESULTS: usize = 100;

const TOKEN_URL: &str = "https://id.twitch.tv/oauth2/token";
const HELIX: &str = "https://api.twitch.tv/helix/";
/// How long before its expiry a token is dropped.
const TOKEN_MARGIN: Duration = Duration::MINUTE;
/// The longest a token is kept, whatever its expiry.
const MAX_TOKEN_TTL: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// An app access token.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Token {
    pub(super) access_token: String,
    /// When it stops being used.
    pub(super) expires: Time,
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Token")
            .field("expires", &self.expires)
            .finish_non_exhaustive()
    }
}

/// The client credentials grant, as a form body.
pub(super) fn token_request(c: &TwitchCredentials) -> HttpRequest {
    let form = query(&[
        ("client_id", c.client_id.as_str()),
        ("client_secret", c.client_secret.as_str()),
        ("grant_type", "client_credentials"),
    ]);
    HttpRequest::new(Method::Post, TOKEN_URL, MAX_RESPONSE_BYTES)
        .body("application/x-www-form-urlencoded", form.into_bytes())
}

/// The token of a grant made at `now`, used until [`TOKEN_MARGIN`]
/// before it expires.
pub(super) fn parse_token(body: &[u8], now: Time) -> Result<Token, LoadError> {
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Grant {
        access_token: Text,
        expires_in: Count,
    }
    let g: Grant = serde_json::from_slice(body).map_err(parse_error)?;
    if g.access_token.as_str().is_empty() {
        return Err(LoadError::Parse("no access token".into()));
    }
    let ttl =
        Duration::from_secs(i64::try_from(g.expires_in.0).unwrap_or(i64::MAX)).min(MAX_TOKEN_TTL);
    Ok(Token {
        access_token: g.access_token.0,
        expires: now + ttl - TOKEN_MARGIN,
    })
}

/// A Helix request of `path` for each of `values` as `key`.
fn helix<V: AsRef<str>>(
    c: &TwitchCredentials,
    token: &str,
    path: &str,
    key: &str,
    values: &[V],
) -> HttpRequest {
    let first = MAX_RESULTS.to_string();
    let mut pairs: Vec<(&str, &str)> = values.iter().map(|v| (key, v.as_ref())).collect();
    pairs.push(("first", &first));
    let url = format!("{HELIX}{path}?{}", query(&pairs));
    HttpRequest::new(Method::Get, url, MAX_RESPONSE_BYTES)
        .header("Authorization", &format!("Bearer {token}"))
        .header("Client-Id", &c.client_id)
}

pub(super) fn users_request<V: AsRef<str>>(
    c: &TwitchCredentials,
    token: &str,
    logins: &[V],
) -> HttpRequest {
    helix(c, token, "users", "login", logins)
}

pub(super) fn streams_request<V: AsRef<str>>(
    c: &TwitchCredentials,
    token: &str,
    logins: &[V],
) -> HttpRequest {
    helix(c, token, "streams", "user_login", logins)
}

pub(super) fn videos_request<V: AsRef<str>>(
    c: &TwitchCredentials,
    token: &str,
    ids: &[V],
) -> HttpRequest {
    helix(c, token, "videos", "id", ids)
}

#[derive(Deserialize)]
#[serde(default, bound = "T: serde::de::DeserializeOwned")]
struct Page<T> {
    data: Items<T>,
}

impl<T> Default for Page<T> {
    fn default() -> Page<T> {
        Page {
            data: Items::default(),
        }
    }
}

fn page<T: serde::de::DeserializeOwned>(body: &[u8]) -> Result<Vec<T>, LoadError> {
    let p: Page<T> = serde_json::from_slice(body).map_err(parse_error)?;
    Ok(p.data.0)
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub(super) struct User {
    id: Text,
    login: Text,
    display_name: Text,
    profile_image_url: Text,
    offline_image_url: Text,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub(super) struct Stream {
    user_id: Text,
    title: Text,
    viewer_count: Count,
    started_at: Text,
    thumbnail_url: Text,
    is_mature: Flag,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub(super) struct Video {
    id: Text,
    user_login: Text,
    title: Text,
    view_count: Count,
    thumbnail_url: Text,
}

pub(super) fn parse_users(body: &[u8]) -> Result<Vec<User>, LoadError> {
    page(body)
}

pub(super) fn parse_streams(body: &[u8]) -> Result<Vec<Stream>, LoadError> {
    page(body)
}

pub(super) fn parse_videos(body: &[u8]) -> Result<Vec<Video>, LoadError> {
    page(body)
}

fn by_login(users: &[User]) -> BTreeMap<String, &User> {
    users
        .iter()
        .map(|u| (u.login.as_str().to_ascii_lowercase(), u))
        .collect()
}

/// The snippets of the stream listings `ids`: each user's channel, live
/// with its stream's details if it has one.
pub(super) fn stream_snippets(
    ids: &[String],
    users: &[User],
    streams: &[Stream],
    now: Time,
) -> Snippets {
    let users = by_login(users);
    let streams: BTreeMap<&str, &Stream> = streams
        .iter()
        .filter(|s| !s.user_id.as_str().is_empty())
        .map(|s| (s.user_id.as_str(), s))
        .collect();
    let mut out = Vec::new();
    for id in ids {
        let Some(u) = users.get(&id.to_ascii_lowercase()) else {
            continue;
        };
        let mut s = ListingSnippet {
            channel_name: clamp(u.display_name.as_str(), MAX_NAME_BYTES),
            thumbnail: url_image(u.offline_image_url.as_str()),
            channel_logo: url_image(u.profile_image_url.as_str()),
            ..Default::default()
        };
        if let Some(st) = streams.get(u.id.as_str()) {
            s.live = true;
            s.user_count = st.viewer_count.0;
            s.start_time = rfc3339::parse(st.started_at.as_str()).unwrap_or(0);
            s.title = clamp(st.title.as_str(), MAX_TITLE_BYTES);
            s.is_mature = st.is_mature.0;
            let thumbnail = st
                .thumbnail_url
                .as_str()
                .replace("{width}", "640")
                .replace("{height}", "360");
            if let Some(t) = url_image(&cache_busted(&thumbnail, now)) {
                s.thumbnail = Some(t);
            }
        }
        out.push((id.clone(), s));
    }
    out
}

/// The logins of the channels of the videos `ids` names, once each.
pub(super) fn logins(ids: &[String], videos: &[Video]) -> Vec<String> {
    let mut out: Vec<String> = videos
        .iter()
        .filter(|v| ids.iter().any(|id| id == v.id.as_str()))
        .map(|v| v.user_login.as_str().to_ascii_lowercase())
        .filter(|l| !l.is_empty())
        .collect();
    out.sort_unstable();
    out.dedup();
    out.truncate(MAX_RESULTS);
    out
}

/// The snippets of the VOD listings `ids`, with their channels from
/// `users`.
pub(super) fn vod_snippets(ids: &[String], videos: &[Video], users: &[User]) -> Snippets {
    let users = by_login(users);
    let mut out = Vec::new();
    for id in ids {
        let Some(v) = videos.iter().find(|v| v.id.as_str() == id) else {
            continue;
        };
        let thumbnail = v
            .thumbnail_url
            .as_str()
            .replace("%{width}", "640")
            .replace("%{height}", "360");
        let mut s = ListingSnippet {
            user_count: v.view_count.0,
            title: clamp(v.title.as_str(), MAX_TITLE_BYTES),
            thumbnail: url_image(&thumbnail),
            ..Default::default()
        };
        if let Some(u) = users.get(&v.user_login.as_str().to_ascii_lowercase()) {
            s.channel_name = clamp(u.display_name.as_str(), MAX_NAME_BYTES);
            s.channel_logo = url_image(u.profile_image_url.as_str());
        }
        out.push((id.clone(), s));
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use strims_proto::network::v1::directory::ListingSnippetImage;
    use strims_proto::network::v1::directory::listing_snippet_image::SourceOneof;

    use super::*;

    fn creds() -> TwitchCredentials {
        TwitchCredentials {
            client_id: "cid".into(),
            client_secret: "s&cret".into(),
        }
    }

    fn url(u: &str) -> Option<ListingSnippetImage> {
        Some(ListingSnippetImage {
            source_oneof: Some(SourceOneof::Url(u.into())),
        })
    }

    const NOW: Time = Time::from_unix(1_700_000_000, 0);

    #[test]
    fn the_token_grant_is_a_form_post() {
        let r = token_request(&creds());
        assert_eq!(r.method, Method::Post);
        assert_eq!(r.url, TOKEN_URL);
        assert_eq!(
            String::from_utf8(r.body.clone()).unwrap(),
            "client_id=cid&client_secret=s%26cret&grant_type=client_credentials"
        );
        assert_eq!(
            r.headers,
            [(
                "Content-Type".to_owned(),
                "application/x-www-form-urlencoded".to_owned()
            )]
        );
        assert_eq!(r.check(), Ok(()));
    }

    #[test]
    fn a_token_is_kept_until_a_minute_before_it_expires() {
        let t = parse_token(
            br#"{"access_token":"abc","expires_in":5000,"token_type":"bearer"}"#,
            NOW,
        )
        .unwrap();
        assert_eq!(t.access_token, "abc");
        assert_eq!(t.expires, NOW + Duration::from_secs(5000 - 60));
        let short = parse_token(br#"{"access_token":"abc","expires_in":30}"#, NOW).unwrap();
        assert!(short.expires < NOW);
        let long = parse_token(br#"{"access_token":"abc","expires_in":1e300}"#, NOW).unwrap();
        assert_eq!(long.expires, NOW + MAX_TOKEN_TTL - TOKEN_MARGIN);
        assert!(parse_token(br#"{"expires_in":10}"#, NOW).is_err());
        assert!(parse_token(b"nope", NOW).is_err());
    }

    #[test]
    fn helix_requests_encode_their_values_and_ask_for_a_full_page() {
        let r = users_request(&creds(), "tok", &["a", "B_c"]);
        assert_eq!(
            r.url,
            "https://api.twitch.tv/helix/users?login=a&login=B_c&first=100"
        );
        assert_eq!(
            r.headers,
            [
                ("Authorization".to_owned(), "Bearer tok".to_owned()),
                ("Client-Id".to_owned(), "cid".to_owned())
            ]
        );
        let r = streams_request(&creds(), "tok", &["x&first=1"]);
        assert_eq!(
            r.url,
            "https://api.twitch.tv/helix/streams?user_login=x%26first%3D1&first=100"
        );
        let r = videos_request(&creds(), "tok", &["1", "2"]);
        assert_eq!(
            r.url,
            "https://api.twitch.tv/helix/videos?id=1&id=2&first=100"
        );
        assert_eq!(r.max_response_bytes, MAX_RESPONSE_BYTES);
        let injected = users_request(&creds(), "tok\r\nX-Evil: 1", &["a"]);
        assert!(injected.check().is_err());
    }

    const USERS: &str = r#"{"data":[
        {"id":"1","login":"destiny","display_name":"Destiny","type":"","broadcaster_type":"partner",
         "description":"d","profile_image_url":"https://x/logo.png","offline_image_url":"https://x/off.png",
         "view_count":0,"created_at":"2011-01-01T00:00:00Z"},
        {"id":"2","login":"shroud","display_name":"shroud","profile_image_url":"https://x/s.png","offline_image_url":""},
        {"id":null,"login":5}
    ]}"#;

    const STREAMS: &str = r#"{"data":[
        {"id":"9","user_id":"1","user_login":"destiny","user_name":"Destiny","game_id":"1","game_name":"Just Chatting",
         "type":"live","title":"hello","viewer_count":1234,"started_at":"2023-11-14T22:13:20Z","language":"en",
         "thumbnail_url":"https://x/live_user_destiny-{width}x{height}.jpg","tag_ids":[],"tags":["a"],"is_mature":true}
    ],"pagination":{}}"#;

    #[test]
    fn a_stream_snippet_is_its_channel_and_live_stream() {
        let users = parse_users(USERS.as_bytes()).unwrap();
        let streams = parse_streams(STREAMS.as_bytes()).unwrap();
        let ids: Vec<String> = ["Destiny", "shroud", "nobody"].map(String::from).to_vec();
        let got = stream_snippets(&ids, &users, &streams, NOW);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].0, "Destiny");
        assert_eq!(
            got[0].1,
            ListingSnippet {
                title: "hello".into(),
                channel_name: "Destiny".into(),
                user_count: 1234,
                live: true,
                is_mature: true,
                thumbnail: url("https://x/live_user_destiny-640x360.jpg?_t=6553f100"),
                channel_logo: url("https://x/logo.png"),
                start_time: 1_700_000_000,
                ..Default::default()
            }
        );
        assert_eq!(
            got[1],
            (
                "shroud".to_owned(),
                ListingSnippet {
                    channel_name: "shroud".into(),
                    channel_logo: url("https://x/s.png"),
                    ..Default::default()
                }
            )
        );
    }

    const VIDEOS: &str = r#"{"data":[
        {"id":"1157956746","stream_id":null,"user_id":"1","user_login":"Destiny","user_name":"Destiny",
         "title":"vod","description":"","created_at":"2021-09-24T00:00:00Z","url":"https://x",
         "thumbnail_url":"https://x/vod-%{width}x%{height}.jpg","viewable":"public","view_count":"77",
         "language":"en","type":"archive","duration":"1h"},
        {"id":"999","user_login":"other"}
    ]}"#;

    #[test]
    fn a_vod_snippet_is_its_video_and_channel() {
        let videos = parse_videos(VIDEOS.as_bytes()).unwrap();
        let ids = vec!["1157956746".to_owned(), "5".to_owned()];
        assert_eq!(logins(&ids, &videos), ["destiny"]);
        let users = parse_users(USERS.as_bytes()).unwrap();
        let got = vod_snippets(&ids, &videos, &users);
        assert_eq!(
            got,
            [(
                "1157956746".to_owned(),
                ListingSnippet {
                    title: "vod".into(),
                    user_count: 77,
                    thumbnail: url("https://x/vod-640x360.jpg"),
                    channel_name: "Destiny".into(),
                    channel_logo: url("https://x/logo.png"),
                    ..Default::default()
                }
            )]
        );
    }

    #[test]
    fn long_titles_are_clamped() {
        let body = format!(
            r#"{{"data":[{{"id":"1","user_login":"a","title":"{}"}}]}}"#,
            "t".repeat(5000)
        );
        let videos = parse_videos(body.as_bytes()).unwrap();
        let got = vod_snippets(&["1".to_owned()], &videos, &[]);
        assert_eq!(got[0].1.title.len(), MAX_TITLE_BYTES);
    }

    #[test]
    fn pages_without_data_have_no_users_and_non_json_fails() {
        assert!(parse_users(b"nope").is_err());
        assert_eq!(parse_users(b"[]").unwrap(), []);
        assert_eq!(parse_users(br#"{"data":null}"#).unwrap(), []);
        assert_eq!(parse_users(br#"{"error":"x"}"#).unwrap(), []);
    }
}
