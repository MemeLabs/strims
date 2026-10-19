// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Media listing snippets on fake nodes (see `strims_testworld`):
//! publishers serve them on the snippet port, the server subscribes to
//! them, and views show those that verify against their swarm.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::rc::Rc;

use futures_util::StreamExt;
use strims_base::time::Time;
use strims_crypto::{Signer, key_from_seed};
use strims_directory::SignedSnippet;
use strims_ppspp::{SwarmId, SwarmOptions, Uri, UriOptions};
use strims_proto::network::v1::directory::listing::{Content, Media};
use strims_proto::network::v1::directory::listing_snippet_image::SourceOneof;
use strims_proto::network::v1::directory::{
    DirectorySnippetService, Listing, ListingSnippet, ListingSnippetImage, SnippetSubscribeRequest,
    SnippetSubscribeResponse, register_directory_snippet_service,
};
use strims_proto::service::{Context, Error as RpcError, ServerStream};
use strims_proto::types::{Image, ImageType};
use strims_rpc::Router;
use strims_rpc::Timer;
use strims_rpc::vpn::{self, VpnConfig};
use strims_testworld::*;
use strims_transfer::{SwarmHost, TransferId};
use tokio::task::spawn_local;

fn media(id: &SwarmId) -> Listing {
    Listing {
        content: Some(Content::Media(Media {
            mime_type: "video/mp4".into(),
            swarm_uri: Uri::new(id.clone(), UriOptions::default()).to_string(),
        })),
    }
}

fn image(n: u8) -> ListingSnippetImage {
    ListingSnippetImage {
        source_oneof: Some(SourceOneof::Image(Image {
            r#type: ImageType::Png.into(),
            height: 2,
            width: 2,
            data: vec![n; 1024],
        })),
    }
}

fn snippet(title: &str) -> ListingSnippet {
    ListingSnippet {
        title: title.into(),
        live: true,
        channel_name: "chan".into(),
        channel_logo: Some(ListingSnippetImage {
            source_oneof: Some(SourceOneof::Url("https://x/logo.png".into())),
        }),
        ..Default::default()
    }
}

/// The key of swarm `n`.
fn swarm_key(n: u8) -> Signer {
    Signer::new(&key_from_seed(&[n; 32])).unwrap()
}

/// `s` signed now on `node` by the key of swarm `n`.
fn signed(node: &Node, n: u8, s: &ListingSnippet) -> SignedSnippet {
    SignedSnippet::sign(&swarm_key(n), s, node.timer.now())
}

/// A swarm `n` on `node`, with its transfer under the empty salt,
/// published on the network `key` when `publish` is set.
async fn media_swarm(node: &Node, key: &[u8], n: u8, publish: bool) -> (SwarmId, TransferId) {
    let id = SwarmId::new(&swarm_key(n).public());
    let swarm = node
        .host
        .add_swarm(id.clone(), SwarmOptions::default_options())
        .await
        .unwrap();
    let t = node.transfers.add(swarm, &id, Vec::new());
    if publish {
        assert!(node.transfers.publish(t, node.net(key)));
    }
    (id, t)
}

/// The snippet each node's view shows for listing `id` of network
/// `network_id`: none for the empty snippet.
fn shown(nodes: &[&Node], network_id: u64, id: u64) -> Vec<Option<ListingSnippet>> {
    nodes
        .iter()
        .map(|n| {
            n.directory
                .listings(network_id)
                .into_iter()
                .find(|l| l.id == id)
                .and_then(|l| l.snippet)
                .filter(|s| *s != ListingSnippet::default())
        })
        .collect()
}

/// The server shows the snippet the publisher of a media listing serves,
/// the server's own host included, and every view gets it.
#[test]
fn a_published_media_listing_takes_its_publishers_snippet() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let bob = node_in(&world, 3).await;
        let (key, _pipes) = owned_network(&owner, &[&alice, &bob], 30).await;
        let key = key.public;
        let (swarm, _) = media_swarm(&alice, &key, 7, true).await;
        let s = snippet("live now");
        alice
            .directory
            .push_snippet(&swarm, signed(&alice, 7, &s))
            .unwrap();
        let id = alice.directory.publish(&key, media(&swarm)).await.unwrap();

        let nodes = [&owner, &alice, &bob];
        wait_for(10, "the snippet", || {
            shown(&nodes, 30, id).iter().all(|x| x.as_ref() == Some(&s))
        })
        .await;
        let m = alice.directory.metrics(&key).unwrap();
        assert_eq!((m.snippets_served, m.snippets_refused), (1, 0), "{m:?}");

        let (own, _) = media_swarm(&owner, &key, 8, true).await;
        let o = snippet("the owner's");
        owner
            .directory
            .push_snippet(&own, signed(&owner, 8, &o))
            .unwrap();
        let own_id = owner.directory.publish(&key, media(&own)).await.unwrap();
        wait_for(10, "the owner's snippet", || {
            shown(&nodes, 30, own_id)
                .iter()
                .all(|x| x.as_ref() == Some(&o))
        })
        .await;
        let server = owner.directory.metrics(&key).unwrap().server.unwrap();
        assert_eq!(server.snippets_shown, 2, "{server:?}");
    });
}

/// A publisher's later snippets reach the views over the same
/// subscription, both images included.
#[test]
fn snippet_updates_reach_views() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let bob = node_in(&world, 3).await;
        let (key, _pipes) = owned_network(&owner, &[&alice, &bob], 31).await;
        let key = key.public;
        let (swarm, _) = media_swarm(&alice, &key, 7, true).await;
        alice
            .directory
            .push_snippet(&swarm, signed(&alice, 7, &snippet("first")))
            .unwrap();
        let id = alice.directory.publish(&key, media(&swarm)).await.unwrap();
        let nodes = [&owner, &bob];
        wait_for(10, "the first snippet", || {
            shown(&nodes, 31, id)
                .iter()
                .all(|x| x.as_ref().is_some_and(|s| s.title == "first"))
        })
        .await;

        let next = ListingSnippet {
            title: "second".into(),
            thumbnail: Some(image(1)),
            channel_logo: Some(image(2)),
            user_count: 5,
            ..Default::default()
        };
        alice
            .directory
            .push_snippet(&swarm, signed(&alice, 7, &next))
            .unwrap();
        wait_for(10, "the second snippet", || {
            shown(&nodes, 31, id)
                .iter()
                .all(|x| x.as_ref() == Some(&next))
        })
        .await;
        let m = alice.directory.metrics(&key).unwrap();
        assert_eq!(m.snippets_served, 1, "{m:?}");
        let server = owner.directory.metrics(&key).unwrap().server.unwrap();
        assert_eq!(server.snippets_shown, 2, "{server:?}");
    });
}

/// A publisher serves no snippet for a swarm whose transfer it hasn't
/// published on the network; once it does, the server's next attempt
/// takes it.
#[test]
fn an_unpublished_transfer_serves_no_snippet() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (key, _pipes) = owned_network(&owner, &[&alice], 32).await;
        let key = key.public;
        let (swarm, transfer) = media_swarm(&alice, &key, 7, false).await;
        let s = snippet("hidden");
        alice
            .directory
            .push_snippet(&swarm, signed(&alice, 7, &s))
            .unwrap();
        let id = alice.directory.publish(&key, media(&swarm)).await.unwrap();
        sleep_secs(5).await;
        assert_eq!(shown(&[&owner, &alice], 32, id), [None, None]);
        let m = alice.directory.metrics(&key).unwrap();
        assert_eq!((m.snippets_served, m.snippets_refused), (0, 1), "{m:?}");

        assert!(alice.transfers.publish(transfer, alice.net(&key)));
        wait_for(60, "the snippet", || {
            shown(&[&owner, &alice], 32, id)
                .iter()
                .all(|x| x.as_ref() == Some(&s))
        })
        .await;
    });
}

/// Serves snippets signed by a key other than the swarm's.
struct Forged;

impl DirectorySnippetService for Forged {
    async fn subscribe(
        &self,
        _: &Context,
        _: SnippetSubscribeRequest,
    ) -> Result<ServerStream<SnippetSubscribeResponse>, RpcError> {
        let forger = swarm_key(9);
        let responses = ["forged", "forged again", "forged once more"].map(|title| {
            let forged = SignedSnippet::sign(&forger, &snippet(title), Time::from_unix_millis(1));
            let (envelope, images) = forged.into_parts();
            Ok(SnippetSubscribeResponse {
                snippet: Some(envelope),
                image: images
                    .into_iter()
                    .next()
                    .map(|(_, b)| b)
                    .unwrap_or_default(),
            })
        });
        let stream = futures_util::stream::iter(responses).chain(futures_util::stream::pending());
        Ok(Box::pin(stream))
    }
}

/// The server shows no snippet whose signature fails, and gives up on a
/// publisher whose snippets keep failing.
#[test]
fn a_snippet_that_fails_verification_is_not_shown() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let mallory = node_in(&world, 2).await;
        // the control's member port opens elsewhere, so that the forger
        // answers on the snippet port
        mallory.host.state.borrow_mut().port_remap.insert(14, 15);
        let (key, _pipes) = owned_network(&owner, &[&mallory], 33).await;
        let key = key.public;
        let (reader, writer) = mallory.host.open_port(mallory.net(&key), 14).unwrap();
        let mut router = Router::new();
        register_directory_snippet_service(&mut router, Rc::new(Forged));
        let cert = mallory.network.host_certificate(&key).unwrap();
        let (_client, conn) =
            vpn::connect(VpnConfig::default(), &cert, reader, writer, Some(router)).unwrap();
        spawn_local(async move {
            let _ = conn.await;
        });

        let (swarm, _) = media_swarm(&mallory, &key, 7, true).await;
        let id = mallory
            .directory
            .publish(&key, media(&swarm))
            .await
            .unwrap();
        sleep_secs(10).await;
        assert_eq!(shown(&[&owner, &mallory], 33, id), [None, None]);
        assert_eq!(owner.directory.listings(33).len(), 1);
        let server = owner.directory.metrics(&key).unwrap().server.unwrap();
        assert_eq!(server.snippets_shown, 0, "{server:?}");
        assert_eq!(server.snippets_rejected, 1, "{server:?}");
        assert_eq!(server.snippets_unverified, 3, "{server:?}");
    });
}
