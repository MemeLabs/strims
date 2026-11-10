// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The bootstrap API: what a browser needs to open a stream of a network
//! it isn't a member of yet, in one step, served on the HLS listener under
//! [`PREFIX`] for the networks [`BootstrapConfig::networks`] names. Each
//! must be one whose CA the running profile holds (its owner), since its
//! invitations then admit any number of newcomers
//! ([`ca`](strims_network::ca)).
//!
//! - `GET /api/v1/directory/events` streams the networks' media listings,
//!   as [`DirectoryFrontend`]'s `WatchListings` has them: a `Change` per
//!   network with its listings, then their changes. Each message is a
//!   uvarint length and an encoded `FrontendWatchListingsResponse`
//!   (`application/octet-stream`). A snippet's inline raster images
//!   ([`raster_type`]) become `url` sources naming their thumbnail route,
//!   and its other inline images are left out; images that are URLs stay
//!   them. An empty message (one zero byte) follows [`PING_INTERVAL`]
//!   without another, so a client and the proxies before it see the
//!   stream live. While the profile holds none of the networks, the stream
//!   carries pings alone and ends after [`IDLE_STREAM`], so the client's
//!   next one names those that started since. The client reconnects
//!   whenever a stream ends, and starts its view over. At most
//!   [`Bootstrap::max_streams`] are open at once, half the listener's
//!   connections: over HTTP/1.1 each holds one, so the other routes keep
//!   the rest, and a stream past the bound answers 503 rather than finding
//!   the listener full.
//! - `GET /api/v1/directory/thumbnails/<network>/<listing id>/<digest>`
//!   answers the raster image of the listing's snippet whose SHA-256
//!   ([`digest`] of its encoded `ListingSnippetImage`) the path names, in
//!   hex, with its type's MIME and an immutable cache control; 404 for
//!   any other. It reads the one listing from the network's view, so its
//!   work is one image's hash.
//! - `GET /api/v1/invitation/<network>` answers a new invitation to the
//!   network, an encoded `Invitation` (`application/protobuf`), whose one
//!   bootstrap client dials [`BootstrapConfig::ws_url`] and the profile's
//!   key in hex. 404 for a network the API doesn't serve; 503 while the
//!   node doesn't run it yet.
//!
//! A network in a path is its key in unpadded URL-safe base64. Other
//! paths under the prefix answer 404. Every answer allows any origin
//! ([`http`](crate::http)), so a page on another origin reads it with
//! simple `GET`s, and carries [`POLICY`].
//!
//! # Images
//!
//! A snippet's images come from whichever member published its listing,
//! and the API may share an origin with the UI, whose storage holds
//! profile keys. So only raster images, which run no script, are served;
//! with `nosniff` on every answer of the listener, a browser takes each as
//! its type says, and [`POLICY`] sandboxes an answer opened as a page.

use std::cell::Cell;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context as TaskContext, Poll};
use std::time::Duration as StdDuration;

use bytes::Bytes;
use data_encoding::{BASE64URL_NOPAD, HEXLOWER, HEXLOWER_PERMISSIVE};
use futures_util::future::LocalBoxFuture;
use futures_util::{FutureExt, Stream, stream};
use prost::Message;
use strims_directory::DirectoryFrontend;
use strims_directory::snippet::digest;
use strims_proto::network::v1::Network;
use strims_proto::network::v1::bootstrap::bootstrap_client::ClientOptions;
use strims_proto::network::v1::bootstrap::{BootstrapClient, BootstrapClientWebSocketOptions};
use strims_proto::network::v1::directory::frontend_watch_listings_response::event::Event as WatchEvent;
use strims_proto::network::v1::directory::listing_snippet_image::SourceOneof;
use strims_proto::network::v1::directory::{
    DirectoryFrontendService as _, FrontendWatchListingsRequest, FrontendWatchListingsResponse,
    ListingContentType, ListingQuery, ListingSnippet, ListingSnippetImage, listing_query,
};
use strims_proto::service::{Context, ServerStream};
use strims_proto::types::ImageType;
use strims_video::hls::HlsResponse;
use tokio::time::{Instant, Sleep, sleep};

use crate::http::{Answer, ApiRoutes, BodyStream};
use crate::{BootstrapConfig, SvcRuntime};

/// The paths the bootstrap API answers start with this.
pub(crate) const PREFIX: &str = "/api/v1/";

/// The content security policy of the API's answers: an answer opened as
/// a page loads nothing and runs in a sandbox, without its origin.
pub(crate) const POLICY: &str = "default-src 'none'; sandbox";

const EVENTS: &str = "directory/events";
const THUMBNAILS: &str = "directory/thumbnails/";
const INVITATION: &str = "invitation/";

/// How long an event stream goes without a message before it sends an
/// empty one.
pub(crate) const PING_INTERVAL: StdDuration = StdDuration::from_secs(15);

/// How long an event stream that names no network lasts.
pub(crate) const IDLE_STREAM: StdDuration = StdDuration::from_secs(60);

const EVENTS_TYPE: &str = "application/octet-stream";
const INVITATION_TYPE: &str = "application/protobuf";
const NO_STORE: &str = "no-store";
const IMMUTABLE: &str = "public, max-age=31536000, immutable";

/// Whether the bootstrap API answers `path`, its query included.
pub(crate) fn routes(path: &str) -> bool {
    path.starts_with(PREFIX)
}

/// The bootstrap API of a running profile.
pub(crate) struct Bootstrap {
    runtime: Rc<SvcRuntime>,
    /// The keys of the networks it serves. Bound: the config's.
    networks: Vec<Vec<u8>>,
    /// The URL its invitations' bootstrap client dials.
    client_url: String,
    /// The event streams open. Bound: `max_streams`.
    streams: Rc<Cell<usize>>,
    /// The most event streams open at once: [`max_streams`].
    max_streams: usize,
}

/// The most event streams open at once on a listener that serves at most
/// `connections`: half of them, and one at least.
pub(crate) fn max_streams(connections: usize) -> usize {
    (connections / 2).max(1)
}

impl Bootstrap {
    /// The API of the profile whose public key is `profile_key`, running
    /// as `runtime`, under `config`, on a listener that serves at most
    /// `connections` at once.
    pub(crate) fn new(
        runtime: Rc<SvcRuntime>,
        profile_key: &[u8],
        config: &BootstrapConfig,
        connections: usize,
    ) -> Bootstrap {
        Bootstrap {
            runtime,
            networks: config.networks.clone(),
            client_url: client_url(&config.ws_url, profile_key),
            streams: Rc::new(Cell::new(0)),
            max_streams: max_streams(connections),
        }
    }

    /// The record of network `key` if the API serves it: `Err(404)` for
    /// one it doesn't, or whose CA the profile no longer holds; `Err(503)`
    /// while the node doesn't run it.
    fn owned(&self, key: &[u8]) -> Result<Network, u16> {
        if !self.networks.iter().any(|k| k == key) {
            return Err(404);
        }
        let info = self.runtime.network().network(key).ok_or(503u16)?;
        if info.network.server_config.is_none() {
            return Err(404);
        }
        Ok(info.network)
    }

    async fn answer(&self, path: &str, body: bool) -> Answer {
        let path = path.split_once('?').map_or(path, |(p, _)| p);
        let Some(route) = path.strip_prefix(PREFIX) else {
            return Answer::error(404);
        };
        let answer = if route == EVENTS {
            self.events(body).await
        } else if let Some(rest) = route.strip_prefix(THUMBNAILS) {
            self.thumbnail(rest)
        } else if let Some(rest) = route.strip_prefix(INVITATION) {
            self.invitation(rest)
        } else {
            Answer::error(404)
        };
        Answer { body, ..answer }
    }

    async fn events(&self, body: bool) -> Answer {
        if self.streams.get() >= self.max_streams {
            return Answer::error(503);
        }
        let head = ok(EVENTS_TYPE, NO_STORE, Rc::from(&[][..]));
        if !body {
            return Answer {
                stream: Some(Box::pin(stream::empty())),
                ..Answer::whole(head, false)
            };
        }
        let keys: Vec<Vec<u8>> = self
            .networks
            .iter()
            .filter(|k| self.owned(k).is_ok())
            .cloned()
            .collect();
        let listings = if keys.is_empty() {
            None
        } else {
            let frontend = DirectoryFrontend::new(self.runtime.directory().clone());
            let req = FrontendWatchListingsRequest {
                content_types: vec![ListingContentType::Media as i32],
                network_keys: keys,
                listing_id: 0,
            };
            match frontend.watch_listings(&Context::default(), req).await {
                Ok(listings) => Some(listings),
                Err(e) => {
                    tracing::debug!(error = %e, "a bootstrap event stream didn't open");
                    return Answer::error(503);
                }
            }
        };
        let events = Events {
            end: listings.is_none().then(|| Box::pin(sleep(IDLE_STREAM))),
            listings,
            ping: Box::pin(sleep(PING_INTERVAL)),
            _open: Open::new(&self.streams),
        };
        Answer {
            stream: Some(Box::pin(events) as BodyStream),
            ..Answer::whole(head, true)
        }
    }

    fn thumbnail(&self, rest: &str) -> Answer {
        let Some((key, id, want)) = thumbnail_path(rest) else {
            return Answer::error(404);
        };
        if let Err(status) = self.owned(&key) {
            return Answer::error(status);
        }
        let directory = self.runtime.directory();
        let Some(network) = directory
            .networks()
            .into_iter()
            .find(|n| n.network_key == key)
        else {
            return Answer::error(404);
        };
        let query = ListingQuery {
            query: Some(listing_query::Query::Id(id)),
        };
        let image = directory
            .listing(network.network_id, &query)
            .and_then(|l| l.snippet)
            .and_then(|s| find_image(s, &want));
        match image {
            Some((ty, data)) => Answer::whole(ok(ty, IMMUTABLE, Rc::from(data)), true),
            None => Answer::error(404),
        }
    }

    fn invitation(&self, rest: &str) -> Answer {
        let Ok(key) = BASE64URL_NOPAD.decode(rest.as_bytes()) else {
            return Answer::error(404);
        };
        let network = match self.owned(&key) {
            Ok(n) => n,
            Err(status) => return Answer::error(status),
        };
        let client = BootstrapClient {
            client_options: Some(ClientOptions::WebsocketOptions(
                BootstrapClientWebSocketOptions {
                    url: self.client_url.clone(),
                    insecure_skip_verify_tls: false,
                },
            )),
            ..Default::default()
        };
        match self.runtime.network().invitation(&network, vec![client]) {
            Ok(invitation) => Answer::whole(
                ok(
                    INVITATION_TYPE,
                    NO_STORE,
                    Rc::from(invitation.encode_to_vec()),
                ),
                true,
            ),
            Err(e) => {
                tracing::warn!(error = %e, "minting a bootstrap invitation failed");
                Answer::error(503)
            }
        }
    }
}

impl ApiRoutes for Bootstrap {
    fn request<'a>(&'a self, path: &'a str, body: bool) -> LocalBoxFuture<'a, Answer> {
        self.answer(path, body).boxed_local()
    }
}

fn ok(content_type: &'static str, cache_control: &'static str, body: Rc<[u8]>) -> HlsResponse {
    HlsResponse {
        status: 200,
        content_type,
        cache_control,
        body,
    }
}

/// The URL a bootstrap client dials the node of `profile_key` at, from
/// the WebSocket URL `ws_url` reaches its listener at.
fn client_url(ws_url: &str, profile_key: &[u8]) -> String {
    format!(
        "{}/{}",
        ws_url.trim_end_matches('/'),
        HEXLOWER.encode(profile_key)
    )
}

/// Counts an event stream among those open while it lasts.
struct Open(Rc<Cell<usize>>);

impl Open {
    fn new(count: &Rc<Cell<usize>>) -> Open {
        count.set(count.get().saturating_add(1));
        Open(count.clone())
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        self.0.set(self.0.get().saturating_sub(1));
    }
}

/// An event stream's body: the watch's responses, delimited, and pings.
struct Events {
    /// `None` while the profile holds none of the networks.
    listings: Option<ServerStream<FrontendWatchListingsResponse>>,
    /// When to ping, [`PING_INTERVAL`] after the last message.
    ping: Pin<Box<Sleep>>,
    /// When a stream without listings ends.
    end: Option<Pin<Box<Sleep>>>,
    _open: Open,
}

impl Stream for Events {
    type Item = Bytes;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Bytes>> {
        let this = &mut *self;
        if let Some(listings) = &mut this.listings {
            match listings.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(res))) => {
                    this.sent();
                    return Poll::Ready(Some(frame(&project(res))));
                }
                Poll::Ready(Some(Err(e))) => {
                    tracing::debug!(error = %e, "a bootstrap event stream's watch failed");
                    return Poll::Ready(None);
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => {}
            }
        }
        if let Some(end) = &mut this.end
            && end.as_mut().poll(cx).is_ready()
        {
            return Poll::Ready(None);
        }
        if this.ping.as_mut().poll(cx).is_ready() {
            this.sent();
            return Poll::Ready(Some(frame(&FrontendWatchListingsResponse::default())));
        }
        Poll::Pending
    }
}

impl Events {
    /// Puts the next ping [`PING_INTERVAL`] after now.
    fn sent(&mut self) {
        self.ping.as_mut().reset(Instant::now() + PING_INTERVAL);
    }
}

/// `res`, uvarint-length-delimited.
fn frame(res: &FrontendWatchListingsResponse) -> Bytes {
    Bytes::from(res.encode_length_delimited_to_vec())
}

/// `res` as the event stream carries it: each listing's inline images
/// replaced by their thumbnail route's path.
fn project(mut res: FrontendWatchListingsResponse) -> FrontendWatchListingsResponse {
    for e in &mut res.events {
        let Some(WatchEvent::Change(change)) = &mut e.event else {
            continue;
        };
        let Some(listings) = &mut change.listings else {
            continue;
        };
        let network = listings
            .network
            .as_ref()
            .map(|n| BASE64URL_NOPAD.encode(&n.key))
            .unwrap_or_default();
        for item in &mut listings.listings {
            if let Some(snippet) = &mut item.snippet {
                link_images(snippet, &network, item.id);
            }
        }
    }
    res
}

/// Replaces `snippet`'s inline images with the paths the thumbnail route
/// answers them at, for listing `id` of the network `network` names.
fn link_images(snippet: &mut ListingSnippet, network: &str, id: u64) {
    for image in [&mut snippet.thumbnail, &mut snippet.channel_logo] {
        *image = image.take().and_then(|i| link(i, network, id));
    }
}

/// `image` as a source the event stream carries: an inline raster one as
/// the thumbnail route's path, a URL as it is, and none for another inline
/// one or one without a source.
fn link(image: ListingSnippetImage, network: &str, id: u64) -> Option<ListingSnippetImage> {
    match &image.source_oneof {
        Some(SourceOneof::Image(i)) if raster_type(i.r#type).is_some() => {
            let digest = HEXLOWER.encode(&digest(&image.encode_to_vec()));
            Some(ListingSnippetImage {
                source_oneof: Some(SourceOneof::Url(format!(
                    "{PREFIX}{THUMBNAILS}{network}/{id}/{digest}"
                ))),
            })
        }
        Some(SourceOneof::Url(_)) => Some(image),
        Some(SourceOneof::Image(_)) | None => None,
    }
}

/// The network key, listing id and digest a thumbnail route's path names
/// after its prefix.
fn thumbnail_path(rest: &str) -> Option<(Vec<u8>, u64, Vec<u8>)> {
    let mut parts = rest.split('/');
    let (Some(key), Some(id), Some(digest), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    let key = BASE64URL_NOPAD.decode(key.as_bytes()).ok()?;
    let id = id.parse().ok()?;
    let digest = HEXLOWER_PERMISSIVE.decode(digest.as_bytes()).ok()?;
    Some((key, id, digest))
}

/// The MIME type and bytes of `snippet`'s inline raster image whose digest
/// is `want`.
fn find_image(snippet: ListingSnippet, want: &[u8]) -> Option<(&'static str, Vec<u8>)> {
    [snippet.thumbnail, snippet.channel_logo]
        .into_iter()
        .flatten()
        .find(|i| digest(&i.encode_to_vec()).as_slice() == want)
        .and_then(|i| match i.source_oneof {
            Some(SourceOneof::Image(image)) => Some((raster_type(image.r#type)?, image.data)),
            _ => None,
        })
}

/// The MIME type of a raster image of type `ty`; `None` for SVG, which
/// can run script, and for a type it doesn't know.
fn raster_type(ty: i32) -> Option<&'static str> {
    match ImageType::try_from(ty) {
        Ok(ImageType::Apng) => Some("image/apng"),
        Ok(ImageType::Avif) => Some("image/avif"),
        Ok(ImageType::Gif) => Some("image/gif"),
        Ok(ImageType::Jpeg) => Some("image/jpeg"),
        Ok(ImageType::Png) => Some("image/png"),
        Ok(ImageType::Webp) => Some("image/webp"),
        Ok(ImageType::Svg | ImageType::Undefined) | Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use strims_proto::network::v1::directory::frontend_watch_listings_response::{
        Change, Event, Unpublish,
    };
    use strims_proto::network::v1::directory::{
        Network as DirectoryNetwork, NetworkListings, NetworkListingsItem,
    };
    use strims_proto::types::Image;

    use super::*;

    fn inline(data: &[u8]) -> ListingSnippetImage {
        inline_of(ImageType::Jpeg, data)
    }

    fn inline_of(ty: ImageType, data: &[u8]) -> ListingSnippetImage {
        ListingSnippetImage {
            source_oneof: Some(SourceOneof::Image(Image {
                r#type: ty as i32,
                height: 1,
                width: 1,
                data: data.to_vec(),
            })),
        }
    }

    fn url(u: &str) -> ListingSnippetImage {
        ListingSnippetImage {
            source_oneof: Some(SourceOneof::Url(u.into())),
        }
    }

    #[test]
    fn inline_images_become_their_thumbnail_paths_and_others_stay() {
        let thumbnail = inline(b"jpeg");
        let snippet = ListingSnippet {
            title: "t".into(),
            thumbnail: Some(thumbnail.clone()),
            channel_logo: Some(url("https://img.example/logo.png")),
            ..Default::default()
        };
        let key = vec![7u8; 32];
        let res = FrontendWatchListingsResponse {
            events: vec![
                Event {
                    event: Some(WatchEvent::Change(Change {
                        listings: Some(NetworkListings {
                            network: Some(DirectoryNetwork {
                                id: 1,
                                name: "n".into(),
                                key: key.clone(),
                            }),
                            listings: vec![
                                NetworkListingsItem {
                                    id: 42,
                                    snippet: Some(snippet),
                                    ..Default::default()
                                },
                                NetworkListingsItem {
                                    id: 43,
                                    snippet: Some(ListingSnippet {
                                        thumbnail: Some(ListingSnippetImage::default()),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                },
                            ],
                        }),
                    })),
                },
                Event {
                    event: Some(WatchEvent::Unpublish(Unpublish {
                        network_id: 1,
                        listing_id: 9,
                    })),
                },
            ],
        };
        let out = project(res.clone());
        let WatchEvent::Change(change) = out.events[0].event.clone().unwrap() else {
            unreachable!()
        };
        let items = change.listings.unwrap().listings;
        let net = BASE64URL_NOPAD.encode(&key);
        let hex = HEXLOWER.encode(&digest(&thumbnail.encode_to_vec()));
        let snippet = items[0].snippet.clone().unwrap();
        assert_eq!(
            snippet.thumbnail,
            Some(url(&format!("/api/v1/directory/thumbnails/{net}/42/{hex}")))
        );
        assert_eq!(
            snippet.channel_logo,
            Some(url("https://img.example/logo.png"))
        );
        assert_eq!(snippet.title, "t");
        assert_eq!(items[1].snippet.clone().unwrap().thumbnail, None);
        assert_eq!(out.events[1], res.events[1]);

        // The path the stream names finds the image it replaced.
        let rest = format!("{net}/42/{hex}");
        let (k, id, want) = thumbnail_path(&rest).unwrap();
        assert_eq!((k, id), (key, 42));
        let original = ListingSnippet {
            thumbnail: Some(thumbnail),
            ..Default::default()
        };
        let found = find_image(original.clone(), &want).unwrap();
        assert_eq!(found, ("image/jpeg", b"jpeg".to_vec()));
        assert_eq!(find_image(original, &[0; 32]), None);
    }

    #[test]
    fn images_that_can_run_script_are_neither_linked_nor_served() {
        let svg = inline_of(ImageType::Svg, b"<svg><script>alert(1)</script></svg>");
        let unknown = ListingSnippetImage {
            source_oneof: Some(SourceOneof::Image(Image {
                r#type: 99,
                data: b"?".to_vec(),
                ..Default::default()
            })),
        };
        for image in [svg, unknown, inline_of(ImageType::Undefined, b"?")] {
            let mut snippet = ListingSnippet {
                thumbnail: Some(image.clone()),
                channel_logo: Some(image.clone()),
                ..Default::default()
            };
            link_images(&mut snippet, "net", 1);
            assert_eq!((snippet.thumbnail, snippet.channel_logo), (None, None));
            let held = ListingSnippet {
                thumbnail: Some(image.clone()),
                ..Default::default()
            };
            assert_eq!(find_image(held, &digest(&image.encode_to_vec())), None);
        }
        for ty in [
            ImageType::Apng,
            ImageType::Avif,
            ImageType::Gif,
            ImageType::Jpeg,
            ImageType::Png,
            ImageType::Webp,
        ] {
            let mut snippet = ListingSnippet {
                thumbnail: Some(inline_of(ty, b"x")),
                ..Default::default()
            };
            link_images(&mut snippet, "net", 1);
            assert!(snippet.thumbnail.is_some(), "{ty:?}");
        }
    }

    #[test]
    fn event_streams_take_half_the_listeners_connections() {
        assert_eq!(max_streams(32), 16);
        assert_eq!(max_streams(5), 2);
        assert_eq!(max_streams(1), 1);
        assert_eq!(max_streams(0), 1);
    }

    #[test]
    fn thumbnail_paths_name_a_key_an_id_and_a_digest() {
        let net = BASE64URL_NOPAD.encode(&[1; 32]);
        let hex = "ab".repeat(32);
        assert!(thumbnail_path(&format!("{net}/1/{hex}")).is_some());
        for bad in [
            format!("{net}/1"),
            format!("{net}/1/{hex}/x"),
            format!("{net}/x/{hex}"),
            format!("{net}/1/zz"),
            format!("{net}=/1/{hex}"),
            String::new(),
        ] {
            assert!(thumbnail_path(&bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn messages_are_delimited_and_a_ping_is_one_zero_byte() {
        assert_eq!(&frame(&FrontendWatchListingsResponse::default())[..], &[0]);
        let res = FrontendWatchListingsResponse {
            events: vec![Event::default()],
        };
        let framed = frame(&res);
        let decoded = FrontendWatchListingsResponse::decode_length_delimited(&framed[..]).unwrap();
        assert_eq!(decoded, res);
    }

    #[test]
    fn client_urls_end_in_the_profiles_key() {
        let key = [0xab; 2];
        assert_eq!(
            client_url("wss://node.example", &key),
            "wss://node.example/abab"
        );
        assert_eq!(
            client_url("wss://node.example/ws/", &key),
            "wss://node.example/ws/abab"
        );
    }

    #[test]
    fn raster_image_types_map_to_mime_types() {
        assert_eq!(raster_type(ImageType::Jpeg as i32), Some("image/jpeg"));
        assert_eq!(raster_type(ImageType::Webp as i32), Some("image/webp"));
        assert_eq!(raster_type(ImageType::Svg as i32), None);
        assert_eq!(raster_type(99), None);
        assert!(routes("/api/v1/invitation/x"));
        assert!(!routes("/api/v2/invitation/x"));
        assert!(!routes("/hls/x"));
    }
}
