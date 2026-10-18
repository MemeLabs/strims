// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! YouTube videos, from the Data API v3 with a public API key.

use serde::Deserialize;
use strims_proto::network::v1::directory::ListingSnippet;

use super::{
    Count, Items, LoadError, MAX_NAME_BYTES, MAX_RESPONSE_BYTES, MAX_TITLE_BYTES, Or, Snippets,
    Text, clamp, parse_error, rfc3339, url_image,
};
use crate::fetch::{HttpRequest, Method, query};

/// The most ids a request takes, and the page size it asks for.
pub(super) const MAX_RESULTS: usize = 50;

const API: &str = "https://www.googleapis.com/youtube/v3/";

fn request<V: AsRef<str>>(key: &str, path: &str, part: &str, ids: &[V]) -> HttpRequest {
    let ids: Vec<&str> = ids.iter().map(AsRef::as_ref).collect();
    let max = MAX_RESULTS.to_string();
    let q = query(&[
        ("part", part),
        ("id", &ids.join(",")),
        ("key", key),
        ("maxResults", &max),
    ]);
    HttpRequest::new(Method::Get, format!("{API}{path}?{q}"), MAX_RESPONSE_BYTES)
}

pub(super) fn videos_request<V: AsRef<str>>(key: &str, ids: &[V]) -> HttpRequest {
    let part = "id,liveStreamingDetails,snippet,statistics,contentDetails";
    request(key, "videos", part, ids)
}

pub(super) fn channels_request<V: AsRef<str>>(key: &str, ids: &[V]) -> HttpRequest {
    request(key, "channels", "id,snippet", ids)
}

#[derive(Deserialize)]
#[serde(default, bound = "T: serde::de::DeserializeOwned")]
struct List<T> {
    items: Items<T>,
}

impl<T> Default for List<T> {
    fn default() -> List<T> {
        List {
            items: Items::default(),
        }
    }
}

fn list<T: serde::de::DeserializeOwned>(body: &[u8]) -> Result<Vec<T>, LoadError> {
    let l: List<T> = serde_json::from_slice(body).map_err(parse_error)?;
    Ok(l.items.0)
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
struct Thumbnail {
    url: Text,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
struct Thumbnails {
    medium: Or<Thumbnail>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
struct VideoSnippet {
    title: Text,
    channel_id: Text,
    thumbnails: Or<Thumbnails>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
struct ContentRating {
    yt_rating: Text,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
struct ContentDetails {
    content_rating: Or<ContentRating>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
struct LiveDetails {
    actual_start_time: Text,
    actual_end_time: Text,
    concurrent_viewers: Count,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
struct Statistics {
    view_count: Count,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub(super) struct Video {
    id: Text,
    snippet: Or<VideoSnippet>,
    content_details: Or<ContentDetails>,
    /// Present for a live stream, past or present.
    live_streaming_details: Option<Or<LiveDetails>>,
    statistics: Or<Statistics>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
struct ChannelSnippet {
    title: Text,
    thumbnails: Or<Thumbnails>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub(super) struct Channel {
    id: Text,
    snippet: Or<ChannelSnippet>,
}

pub(super) fn parse_videos(body: &[u8]) -> Result<Vec<Video>, LoadError> {
    list(body)
}

pub(super) fn parse_channels(body: &[u8]) -> Result<Vec<Channel>, LoadError> {
    list(body)
}

/// The channels of the videos `ids` names, once each.
pub(super) fn channel_ids(ids: &[String], videos: &[Video]) -> Vec<String> {
    let mut out: Vec<String> = videos
        .iter()
        .filter(|v| ids.iter().any(|id| id == v.id.as_str()))
        .map(|v| v.snippet.0.channel_id.0.clone())
        .filter(|c| !c.is_empty())
        .collect();
    out.sort_unstable();
    out.dedup();
    out.truncate(MAX_RESULTS);
    out
}

/// The snippets of the video listings `ids`, with their channels from
/// `channels`: live with the concurrent viewers while a stream hasn't
/// ended, else with the video's views.
pub(super) fn snippets(ids: &[String], videos: &[Video], channels: &[Channel]) -> Snippets {
    let mut out = Vec::new();
    for id in ids {
        let Some(v) = videos.iter().find(|v| v.id.as_str() == id) else {
            continue;
        };
        let vs = &v.snippet.0;
        let mut s = ListingSnippet {
            title: clamp(vs.title.as_str(), MAX_TITLE_BYTES),
            is_mature: v.content_details.0.content_rating.0.yt_rating.as_str() == "ytAgeRestricted",
            thumbnail: url_image(vs.thumbnails.0.medium.0.url.as_str()),
            ..Default::default()
        };
        if let Some(c) = channels
            .iter()
            .find(|c| !c.id.as_str().is_empty() && c.id.as_str() == vs.channel_id.as_str())
        {
            s.channel_name = clamp(c.snippet.0.title.as_str(), MAX_NAME_BYTES);
            s.channel_logo = url_image(c.snippet.0.thumbnails.0.medium.0.url.as_str());
        }
        match &v.live_streaming_details {
            Some(Or(live)) if live.actual_end_time.as_str().is_empty() => {
                s.live = true;
                s.user_count = live.concurrent_viewers.0;
                s.start_time = rfc3339::parse(live.actual_start_time.as_str()).unwrap_or(0);
            }
            _ => s.user_count = v.statistics.0.view_count.0,
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

    fn url(u: &str) -> Option<ListingSnippetImage> {
        Some(ListingSnippetImage {
            source_oneof: Some(SourceOneof::Url(u.into())),
        })
    }

    #[test]
    fn requests_join_ids_and_encode_every_value() {
        let r = videos_request("k&ey", &["DDTGlyJVNVI", "-IO6fpjDJY8"]);
        assert_eq!(
            r.url,
            "https://www.googleapis.com/youtube/v3/videos?part=id%2CliveStreamingDetails%2Csnippet%2Cstatistics%2CcontentDetails&id=DDTGlyJVNVI%2C-IO6fpjDJY8&key=k%26ey&maxResults=50"
        );
        assert!(r.headers.is_empty());
        let r = channels_request("k", &["UC1"]);
        assert_eq!(
            r.url,
            "https://www.googleapis.com/youtube/v3/channels?part=id%2Csnippet&id=UC1&key=k&maxResults=50"
        );
    }

    const VIDEOS: &str = r#"{"kind":"youtube#videoListResponse","items":[
        {"kind":"youtube#video","id":"DDTGlyJVNVI",
         "snippet":{"publishedAt":"2020-01-01T00:00:00Z","channelId":"UC1","title":"live now",
           "thumbnails":{"default":{"url":"https://i/d.jpg"},"medium":{"url":"https://i/m.jpg","width":320}},
           "liveBroadcastContent":"live"},
         "contentDetails":{"duration":"P0D","contentRating":{"ytRating":"ytAgeRestricted"}},
         "statistics":{"viewCount":"100"},
         "liveStreamingDetails":{"actualStartTime":"2023-11-14T22:13:20.5Z","concurrentViewers":"42"}},
        {"id":"SocSlBubzwA",
         "snippet":{"channelId":"UC2","title":"old stream","thumbnails":{"medium":{"url":"https://i/o.jpg"}}},
         "contentDetails":{"contentRating":{}},
         "statistics":{"viewCount":"9000"},
         "liveStreamingDetails":{"actualStartTime":"2020-01-01T00:00:00Z","actualEndTime":"2020-01-01T01:00:00Z"}},
        {"id":"nKrznsPB5t8","snippet":null,"statistics":{"viewCount":7}},
        {"id":"notasked___","snippet":{"channelId":"UC3"}}
    ]}"#;

    const CHANNELS: &str = r#"{"items":[
        {"id":"UC1","snippet":{"title":"Chan","thumbnails":{"medium":{"url":"https://i/c.jpg"}}}},
        {"id":"UC2","snippet":{"title":"Other"}}
    ]}"#;

    #[test]
    fn a_video_snippet_is_its_video_and_channel() {
        let videos = parse_videos(VIDEOS.as_bytes()).unwrap();
        let ids: Vec<String> = ["DDTGlyJVNVI", "SocSlBubzwA", "nKrznsPB5t8", "PMesD2l6viA"]
            .map(String::from)
            .to_vec();
        assert_eq!(channel_ids(&ids, &videos), ["UC1", "UC2"]);
        let channels = parse_channels(CHANNELS.as_bytes()).unwrap();
        let got = snippets(&ids, &videos, &channels);
        assert_eq!(got.len(), 3);
        assert_eq!(
            got[0],
            (
                "DDTGlyJVNVI".to_owned(),
                ListingSnippet {
                    title: "live now".into(),
                    is_mature: true,
                    thumbnail: url("https://i/m.jpg"),
                    channel_name: "Chan".into(),
                    channel_logo: url("https://i/c.jpg"),
                    live: true,
                    user_count: 42,
                    start_time: 1_700_000_000,
                    ..Default::default()
                }
            )
        );
        assert_eq!(
            got[1].1,
            ListingSnippet {
                title: "old stream".into(),
                thumbnail: url("https://i/o.jpg"),
                channel_name: "Other".into(),
                user_count: 9000,
                ..Default::default()
            }
        );
        assert_eq!(
            got[2].1,
            ListingSnippet {
                user_count: 7,
                ..Default::default()
            }
        );
    }
}
