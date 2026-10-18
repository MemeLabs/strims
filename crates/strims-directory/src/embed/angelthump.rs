// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! AngelThump streams, one username per request.

use serde::Deserialize;
use strims_base::time::Time;
use strims_proto::network::v1::directory::ListingSnippet;

use super::{
    Count, Flag, Items, LoadError, MAX_NAME_BYTES, MAX_RESPONSE_BYTES, MAX_TITLE_BYTES, Or,
    Snippets, Text, cache_busted, clamp, parse_error, rfc3339, url_image,
};
use crate::fetch::{HttpRequest, Method, query};

const API: &str = "https://api.angelthump.com/v3/streams/";

pub(super) fn request(username: &str) -> HttpRequest {
    let url = format!("{API}?{}", query(&[("username", username)]));
    HttpRequest::new(Method::Get, url, MAX_RESPONSE_BYTES)
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
struct User {
    display_name: Text,
    offline_banner_url: Text,
    profile_logo_url: Text,
    title: Text,
    nsfw: Flag,
    username: Text,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub(super) struct Stream {
    #[serde(rename = "type")]
    kind: Text,
    thumbnail_url: Text,
    viewer_count: Count,
    #[serde(rename = "createdAt")]
    created_at: Text,
    user: Or<User>,
}

/// The streams of a response: a JSON array.
pub(super) fn parse(body: &[u8]) -> Result<Vec<Stream>, LoadError> {
    let streams: Items<Stream> = serde_json::from_slice(body).map_err(parse_error)?;
    Ok(streams.0)
}

/// The snippets of the listings `ids`, each from the stream of the user
/// whose username equals it in any case. A stream without a thumbnail
/// shows the user's offline banner.
pub(super) fn snippets(ids: &[String], streams: &[Stream], now: Time) -> Snippets {
    let mut out = Vec::new();
    for id in ids {
        let Some(st) = streams
            .iter()
            .find(|s| s.user.0.username.as_str().eq_ignore_ascii_case(id))
        else {
            continue;
        };
        let u = &st.user.0;
        let thumbnail = if st.thumbnail_url.as_str().is_empty() {
            url_image(u.offline_banner_url.as_str())
        } else {
            url_image(&cache_busted(st.thumbnail_url.as_str(), now))
        };
        let s = ListingSnippet {
            live: st.kind.as_str() == "live",
            user_count: st.viewer_count.0,
            title: clamp(u.title.as_str(), MAX_TITLE_BYTES),
            is_mature: u.nsfw.0,
            channel_name: clamp(u.display_name.as_str(), MAX_NAME_BYTES),
            thumbnail,
            channel_logo: url_image(u.profile_logo_url.as_str()),
            start_time: rfc3339::parse(st.created_at.as_str()).unwrap_or(0),
            ..Default::default()
        };
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

    fn url(u: &str) -> Option<ListingSnippetImage> {
        Some(ListingSnippetImage {
            source_oneof: Some(SourceOneof::Url(u.into())),
        })
    }

    const NOW: Time = Time::from_unix(1_700_000_000, 0);

    #[test]
    fn the_request_names_one_encoded_username() {
        assert_eq!(
            request("psrngafk").url,
            "https://api.angelthump.com/v3/streams/?username=psrngafk"
        );
        assert_eq!(
            request("a&b").url,
            "https://api.angelthump.com/v3/streams/?username=a%26b"
        );
    }

    const LIVE: &str = r#"[{"id":"x","type":"live","thumbnail_url":"https://thumb/psrngafk.jpeg",
        "viewer_count":12,"createdAt":"2023-11-14T22:13:20.000Z",
        "user":{"id":"u","display_name":"PSRNGAFK","offline_banner_url":"https://b/off.png",
          "profile_logo_url":"https://b/logo.png","title":"title","nsfw":true,"username":"psrngafk"}}]"#;

    #[test]
    fn a_stream_snippet_is_its_stream_and_user() {
        let streams = parse(LIVE.as_bytes()).unwrap();
        let got = snippets(&["PsrngAFK".to_owned()], &streams, NOW);
        assert_eq!(
            got,
            [(
                "PsrngAFK".to_owned(),
                ListingSnippet {
                    live: true,
                    user_count: 12,
                    title: "title".into(),
                    is_mature: true,
                    channel_name: "PSRNGAFK".into(),
                    thumbnail: url("https://thumb/psrngafk.jpeg?_t=6553f100"),
                    channel_logo: url("https://b/logo.png"),
                    start_time: 1_700_000_000,
                    ..Default::default()
                }
            )]
        );
        assert!(snippets(&["other".to_owned()], &streams, NOW).is_empty());
    }

    #[test]
    fn a_stream_without_a_thumbnail_shows_the_offline_banner() {
        let body = r#"[{"type":"offline","thumbnail_url":"","user":{"username":"a","offline_banner_url":"https://b/off.png"}}]"#;
        let streams = parse(body.as_bytes()).unwrap();
        let got = snippets(&["a".to_owned()], &streams, NOW);
        assert_eq!(got[0].1.thumbnail, url("https://b/off.png"));
        assert!(!got[0].1.live);
        assert_eq!(parse(b"{}").unwrap(), []);
        assert!(parse(b"").is_err());
    }
}
