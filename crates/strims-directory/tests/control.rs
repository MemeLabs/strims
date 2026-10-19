// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Directory controls on fake nodes (see `strims_testworld`): serving and
//! reading a network's directory, sessions, server config changes, the
//! lease, moderation, asset bundles and aliases.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::Cell;
use std::rc::Rc;

use futures_util::future::LocalBoxFuture;
use strims_base::rng::det_rng;
use strims_dao::Table;
use strims_dao::schema::directory::{DirectoryListingRecords, DirectoryUserRecords, listing_key};
use strims_dao::schema::network::network_scoped_key;
use strims_directory::{
    CallFailure, DirectoryError, DirectoryEvent, EmbedService, Fetch, FetchError, HttpFetch,
    HttpRequest, HttpResponse, ListingEvent, UserEventKind, embed,
};
use strims_network::membership::new_network;
use strims_ppspp::SwarmId;
use strims_proto::network::v1::Network;
use strims_proto::network::v1::directory::listing::{Chat, Content};
use strims_proto::network::v1::directory::listing_query::Query;
use strims_proto::network::v1::directory::server_config::Integrations;
use strims_proto::network::v1::directory::server_config::integrations::AngelThump;
use strims_proto::network::v1::directory::{
    Listing, ListingModeration, ListingQuery, ListingRecord, UserModeration, UserRecord,
};
use strims_proto::types::{Image, ImageType};
use strims_testworld::*;
use strims_transfer::TransferId;

fn chat(n: u8) -> Listing {
    Listing {
        content: Some(Content::Chat(Chat {
            key: vec![n; 32],
            name: format!("chat {n}"),
        })),
    }
}

fn by_id(id: u64) -> ListingQuery {
    ListingQuery {
        query: Some(Query::Id(id)),
    }
}

fn ids(n: &Node, network_id: u64) -> Vec<u64> {
    n.directory
        .listings(network_id)
        .iter()
        .map(|l| l.id)
        .collect()
}

fn ban() -> Option<ListingModeration> {
    Some(ListingModeration {
        is_banned: Some(true),
        ..Default::default()
    })
}

fn moderator() -> Option<UserModeration> {
    Some(UserModeration {
        is_moderator: Some(true),
        ..Default::default()
    })
}

async fn user_record(n: &Node, network_id: u64, peer: &Node) -> Option<UserRecord> {
    let mut tx = n.store.read().await.unwrap();
    let key = network_scoped_key(network_id, &peer.profile_key().public);
    DirectoryUserRecords::index_get(&mut tx, &DirectoryUserRecords::BY_PEER_KEY, &key)
        .await
        .unwrap()
}

async fn listing_record(n: &Node, network_id: u64, l: &Listing) -> Option<ListingRecord> {
    let mut tx = n.store.read().await.unwrap();
    let key = listing_key(network_id, Some(l));
    DirectoryListingRecords::index_get(&mut tx, &DirectoryListingRecords::BY_LISTING, &key)
        .await
        .unwrap()
}

/// Tunables for quick sessions: a two minute timeout, and pings every
/// minute.
fn quick_sessions(n: &mut Network) {
    let d = n
        .server_config
        .as_mut()
        .unwrap()
        .directory
        .get_or_insert_default();
    d.session_timeout = 120;
    d.min_ping_interval = 60;
    d.max_ping_interval = 60;
}

/// A member's published listing reaches every view with its users, and
/// members read the server's swarms through published transfers.
#[test]
fn members_see_a_published_listing_and_its_user_count() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let bob = node_in(&world, 3).await;
        let mut events = bob.directory.subscribe();
        let (key, _pipes) = owned_network(&owner, &[&alice, &bob], 10).await;
        let key = key.public;
        assert!(owner.serving(&key));
        assert!(!alice.serving(&key));
        assert_eq!(
            world.directory_address(&key).map(|a| a.host),
            Some(owner.id)
        );

        let id = alice.directory.publish(&key, chat(1)).await.unwrap();
        assert_eq!(bob.directory.join(&key, by_id(id)).await.unwrap(), id);
        sleep_secs(3).await;
        for n in [&owner, &alice, &bob] {
            let ls = n.directory.listings(10);
            assert_eq!(ls.len(), 1, "{ls:?}");
            assert_eq!((ls[0].id, &ls[0].listing), (id, &chat(1)));
            assert_eq!(ls[0].user_count, 2);
            let mut users: Vec<Vec<u8>> = n
                .directory
                .listing_users(10, id)
                .into_iter()
                .map(|u| u.peer_key)
                .collect();
            users.sort();
            let mut want = vec![
                alice.profile_key().public.clone(),
                bob.profile_key().public.clone(),
            ];
            want.sort();
            assert_eq!(users, want);
        }
        let by_peer = owner
            .directory
            .listings_by_peer_key(&alice.profile_key().public);
        assert_eq!(by_peer.len(), 1);
        assert_eq!(by_peer[0].0, key);

        let got = drain(&mut events);
        assert!(
            matches!(&got[0], DirectoryEvent::Started { network_id: 10, network_key } if *network_key == key),
            "{got:?}"
        );
        assert!(got.iter().any(|e| matches!(e,
            DirectoryEvent::Listing { network_id: 10, event: ListingEvent::Changed(l) } if l.id == id)));
        let joins = got
            .iter()
            .filter(|e| {
                matches!(e, DirectoryEvent::User { listing_id, kind: UserEventKind::Joined, .. } if *listing_id == id)
            })
            .count();
        assert_eq!(joins, 2, "{got:?}");

        let swarm = SwarmId::new(&key);
        for m in [&alice, &bob] {
            let net = m.net(&key);
            for salt in [strims_directory::EVENT_SALT, strims_directory::ASSET_SALT] {
                assert!(m.transfers.is_published(TransferId::new(&swarm, salt), net));
            }
            assert_eq!(m.host.swarms_of(&swarm), (0, 2));
        }
        assert_eq!(owner.host.swarms_of(&swarm), (2, 0));
    });
}

/// A member's pings keep its session, and with it its listing; once it
/// stops pinging, the session expires and the listing goes.
#[test]
fn sessions_expire_without_pings_and_stay_with_them() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (key, _pipes) = owned_network_with(&owner, &[&alice], 11, quick_sessions).await;
        let key = key.public;
        let bundle = alice.directory.networks()[0].asset_bundle.clone().unwrap();
        let client = bundle.directory.unwrap();
        assert_eq!(
            (client.min_ping_interval, client.max_ping_interval),
            (60, 60)
        );

        let id = alice.directory.publish(&key, chat(1)).await.unwrap();
        let before = alice.directory.metrics(&key).unwrap();
        sleep_secs(10 * 60).await;
        let ls = owner.directory.listings(11);
        let counts: Vec<(u64, u32)> = ls.iter().map(|l| (l.id, l.user_count)).collect();
        assert_eq!(counts, [(id, 1)]);
        let m = alice.directory.metrics(&key).unwrap();
        assert!(m.pings >= before.pings + 9, "{m:?}");
        assert_eq!(m.ping_errors, before.ping_errors, "{m:?}");

        alice.stop_directory();
        sleep_secs(4 * 60).await;
        assert!(owner.directory.listings(11).is_empty());
        let server = owner.directory.metrics(&key).unwrap().server.unwrap();
        assert_eq!(server.sessions_expired, 1, "{server:?}");
    });
}

/// A node whose record gains the server config serves the directory: its
/// swarms become writers, it publishes the address, and members read it.
#[test]
fn a_server_config_gained_later_starts_the_server() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let mut rng = det_rng(12);
        let owned = new_network(12, "net", None, &owner.profile, "", T0, &mut rng).unwrap();
        let root = owned.server_config.as_ref().unwrap().key.clone().unwrap();
        let key = root.public.clone();
        let config = owned.server_config.clone();
        owner
            .write_network(Network {
                server_config: None,
                ..owned
            })
            .await;
        settle().await;
        let swarm = SwarmId::new(&key);
        assert!(!owner.serving(&key));
        assert_eq!(owner.host.swarms_of(&swarm), (0, 2));
        assert_eq!(world.directory_address(&key), None);
        assert_eq!(
            owner.directory.publish(&key, chat(1)).await,
            Err(CallFailure::NoServer)
        );

        owner.change_network(12, |n| n.server_config = config).await;
        wait_for(5, "the owner to serve", || owner.serving(&key)).await;
        assert_eq!(owner.host.swarms_of(&swarm), (2, 0));
        assert_eq!(
            world.directory_address(&key).map(|a| a.host),
            Some(owner.id)
        );
        let id = owner.directory.publish(&key, chat(1)).await.unwrap();
        sleep_secs(2).await;
        assert_eq!(ids(&owner, 12), [id]);

        // a member signed by the root reads the directory and calls it
        let alice = node_in(&world, 2).await;
        alice.write_network(signed_member(&root, &alice, 12)).await;
        wait_for(60, "the member's view", || ids(&alice, 12) == [id]).await;
        wait_for(60, "the member's call", || {
            alice.directory.metrics(&key).is_some_and(|m| m.pings > 0)
        })
        .await;
        assert_eq!(alice.directory.join(&key, by_id(id)).await.unwrap(), id);
        sleep_secs(2).await;
        assert_eq!(owner.directory.listings(12)[0].user_count, 2);
    });
}

/// A server whose record loses the server config stops: it withdraws the
/// address, its swarms become readers, its view starts over (from what
/// the event stream still holds), and calls find no server.
#[test]
fn a_server_config_lost_reverts_to_a_reader() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (key, _pipes) = owned_network(&owner, &[&alice], 13).await;
        let key = key.public;
        let id = alice.directory.publish(&key, chat(1)).await.unwrap();
        sleep_secs(2).await;
        assert_eq!(ids(&owner, 13), [id]);
        let mut events = owner.directory.subscribe();

        owner.change_network(13, |n| n.server_config = None).await;
        wait_for(5, "the owner to stop serving", || !owner.serving(&key)).await;
        sleep_secs(1).await;
        let swarm = SwarmId::new(&key);
        assert_eq!(owner.host.swarms_of(&swarm), (0, 2));
        let event_transfer = TransferId::new(&swarm, strims_directory::EVENT_SALT);
        assert!(
            owner
                .transfers
                .is_published(event_transfer, owner.net(&key))
        );
        assert_eq!(world.directory_address(&key), None);
        let m = owner.directory.metrics(&key).unwrap();
        assert_eq!((&m.server, m.view.resets), (&None, 1), "{m:?}");
        let got = drain(&mut events);
        assert!(
            got.iter().any(|e| matches!(e,
                DirectoryEvent::Listing { event: ListingEvent::Unpublished(l), .. } if l.id == id)),
            "{got:?}"
        );
        for n in [&alice, &owner] {
            assert_eq!(
                n.directory.publish(&key, chat(2)).await,
                Err(CallFailure::NoServer)
            );
        }
    });
}

/// Of two processes sharing a profile's store, one serves; when it ends,
/// the other takes over once the lease expires.
#[test]
fn one_process_of_a_shared_store_serves() {
    run(async {
        let world = World::default();
        let first = node_in(&world, 1).await;
        let second = node_sharing(&world, 4, &first);
        let alice = node_in(&world, 2).await;
        let (key, _pipes) = owned_network(&first, &[&alice], 14).await;
        let key = key.public;
        let serving = [first.serving(&key), second.serving(&key)];
        assert_eq!(serving.iter().filter(|s| **s).count(), 1, "{serving:?}");
        let (server, other) = if serving[0] {
            (&first, &second)
        } else {
            (&second, &first)
        };
        assert_eq!(
            world.directory_address(&key).map(|a| a.host),
            Some(server.id)
        );
        let id = alice.directory.publish(&key, chat(1)).await.unwrap();
        sleep_secs(3).await;
        for n in [&first, &second, &alice] {
            assert_eq!(ids(n, 14), [id]);
        }

        server.stop();
        wait_for(60, "the other process to serve", || other.serving(&key)).await;
        assert_eq!(
            world.directory_address(&key).map(|a| a.host),
            Some(other.id)
        );
        let again = alice.directory.publish(&key, chat(1)).await.unwrap();
        sleep_secs(3).await;
        for n in [other, &alice] {
            assert_eq!(ids(n, 14), [again]);
        }
    });
}

/// The network's owner bans a member's listing: it leaves every view, its
/// record holds the ban, and publishing it again is refused.
#[test]
fn an_admin_bans_a_listing() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (key, _pipes) = owned_network(&owner, &[&alice], 15).await;
        let key = key.public;
        let id = alice.directory.publish(&key, chat(1)).await.unwrap();
        sleep_secs(2).await;
        assert_eq!(ids(&alice, 15), [id]);

        owner
            .directory
            .moderate_listing(&key, id, ban())
            .await
            .unwrap();
        sleep_secs(2).await;
        assert!(ids(&alice, 15).is_empty());
        assert!(ids(&owner, 15).is_empty());
        let rec = listing_record(&owner, 15, &chat(1)).await.unwrap();
        assert_ne!(rec.id, 0);
        assert_eq!(rec.moderation.unwrap().is_banned, Some(true));
        assert_eq!(
            alice.directory.publish(&key, chat(1)).await,
            Err(CallFailure::Refused(DirectoryError::ListingBanned))
        );
    });
}

/// A moderator may moderate users, but a grant of moderation it makes
/// leaves the target as it was.
#[test]
fn a_moderator_cannot_grant_moderator() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let bob = node_in(&world, 3).await;
        let (key, _pipes) = owned_network(&owner, &[&alice, &bob], 16).await;
        let key = key.public;
        owner
            .directory
            .moderate_user(&key, &alice.profile_key().public, moderator())
            .await
            .unwrap();

        alice
            .directory
            .moderate_user(&key, &bob.profile_key().public, moderator())
            .await
            .unwrap();
        let rec = user_record(&owner, 16, &bob).await.unwrap();
        assert_eq!(rec.moderation.unwrap_or_default().is_moderator, None);
        let id = alice.directory.publish(&key, chat(1)).await.unwrap();
        assert_eq!(
            bob.directory.moderate_listing(&key, id, ban()).await,
            Err(CallFailure::Refused(DirectoryError::PermissionDenied))
        );

        let no_publish = UserModeration {
            disable_publish: Some(true),
            ..Default::default()
        };
        alice
            .directory
            .moderate_user(&key, &bob.profile_key().public, Some(no_publish))
            .await
            .unwrap();
        assert_eq!(
            bob.directory.publish(&key, chat(2)).await,
            Err(CallFailure::Refused(DirectoryError::PublishDisabled))
        );
    });
}

/// An admin makes a member a moderator, who may then ban listings.
#[test]
fn an_admin_can() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let bob = node_in(&world, 3).await;
        let (key, _pipes) = owned_network(&owner, &[&alice, &bob], 17).await;
        let key = key.public;
        let id = bob.directory.publish(&key, chat(1)).await.unwrap();
        assert_eq!(
            alice.directory.moderate_listing(&key, id, ban()).await,
            Err(CallFailure::Refused(DirectoryError::PermissionDenied))
        );

        owner
            .directory
            .moderate_user(&key, &alice.profile_key().public, moderator())
            .await
            .unwrap();
        let rec = user_record(&owner, 17, &alice).await.unwrap();
        assert_ne!(rec.id, 0);
        assert_eq!(rec.moderation.unwrap().is_moderator, Some(true));
        alice
            .directory
            .moderate_listing(&key, id, ban())
            .await
            .unwrap();
        sleep_secs(2).await;
        assert!(ids(&bob, 17).is_empty());
    });
}

/// The owner, through the record its network's creation stores, is the
/// directory's admin from the start.
#[test]
fn the_owner_is_admin_of_its_network() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (key, _pipes) = owned_network(&owner, &[&alice], 18).await;
        let key = key.public;
        let rec = user_record(&owner, 18, &owner).await.unwrap();
        let m = rec.moderation.unwrap();
        assert_eq!((m.is_admin, m.is_moderator), (Some(true), Some(true)));

        let admin = UserModeration {
            is_admin: Some(true),
            is_moderator: Some(true),
            ..Default::default()
        };
        owner
            .directory
            .moderate_user(&key, &alice.profile_key().public, Some(admin))
            .await
            .unwrap();
        let m = user_record(&owner, 18, &alice)
            .await
            .unwrap()
            .moderation
            .unwrap();
        assert_eq!((m.is_admin, m.is_moderator), (Some(true), Some(true)));
        let id = owner.directory.publish(&key, chat(1)).await.unwrap();
        alice
            .directory
            .moderate_listing(&key, id, ban())
            .await
            .unwrap();
        sleep_secs(2).await;
        assert!(ids(&owner, 18).is_empty());
    });
}

/// Members take the asset bundle the server config makes, and its
/// changes.
#[test]
fn asset_bundles_follow_the_server_config() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (key, _pipes) = owned_network(&owner, &[&alice], 19).await;
        let key = key.public;
        let bundle = alice.directory.networks()[0].asset_bundle.clone().unwrap();
        assert_eq!(bundle.icon, None);
        let client = bundle.directory.unwrap();
        assert_eq!(client.publish_quota, 10);
        assert!(client.integrations.unwrap().swarm);
        let mut events = alice.directory.subscribe();

        let icon = Image {
            r#type: ImageType::Png.into(),
            height: 1,
            width: 1,
            data: vec![7; 64],
        };
        let set = icon.clone();
        owner
            .change_network(19, |n| {
                let c = n.server_config.as_mut().unwrap();
                c.icon = Some(set);
                c.directory.get_or_insert_default().publish_quota = 3;
            })
            .await;
        sleep_secs(2).await;
        let got = drain(&mut events);
        let bundles: Vec<_> = got
            .iter()
            .filter_map(|e| match e {
                DirectoryEvent::AssetBundle {
                    network_id: 19,
                    network_key,
                    bundle,
                } if *network_key == key => Some(bundle.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(bundles.len(), 1, "{got:?}");
        assert_eq!(bundles[0].icon, Some(icon));
        assert_eq!(bundles[0].directory.as_ref().unwrap().publish_quota, 3);
        let cached = alice.directory.asset_bundles();
        assert_eq!(cached, [(19, key.clone(), bundles[0].clone())]);
        assert_eq!(
            owner.directory.networks()[0].asset_bundle.as_ref(),
            Some(&bundles[0])
        );
    });
}

/// A member's renewed alias reaches the users of the listings it is in.
#[test]
fn a_renewed_alias_reaches_presence() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let bob = node_in(&world, 3).await;
        let (key, _pipes) = owned_network(&owner, &[&alice, &bob], 20).await;
        let key = key.public;
        let id = bob.directory.publish(&key, chat(1)).await.unwrap();
        alice.directory.join(&key, by_id(id)).await.unwrap();
        sleep_secs(2).await;
        let alias_of = |n: &Node| {
            n.directory
                .listing_users(20, id)
                .into_iter()
                .find(|u| u.peer_key == alice.profile_key().public)
                .map(|u| u.alias)
        };
        assert_eq!(alias_of(&bob).as_deref(), Some("node-2"));
        let mut events = bob.directory.subscribe();

        alice
            .change_network(20, |n| n.alias = "alicia".into())
            .await;
        settle().await;
        let cert = alice.read_network(20).await.certificate.unwrap();
        assert_eq!(
            strims_crypto::certificate_body(&cert).unwrap().subject,
            "alicia"
        );
        for n in [&owner, &bob] {
            assert_eq!(alias_of(n).as_deref(), Some("alicia"));
        }
        let got = drain(&mut events);
        assert!(
            got.iter().any(|e| matches!(e,
                DirectoryEvent::User { listing_id, kind: UserEventKind::Renamed, user, .. }
                    if *listing_id == id && user.alias == "alicia")),
            "{got:?}"
        );
    });
}

/// Answers AngelThump's stream API with a live stream of any username,
/// counting the requests.
struct FakeAngelThump {
    requests: Rc<Cell<u32>>,
}

impl HttpFetch for FakeAngelThump {
    fn fetch(&self, req: HttpRequest) -> LocalBoxFuture<'static, Result<HttpResponse, FetchError>> {
        self.requests.set(self.requests.get() + 1);
        let prefix = "https://api.angelthump.com/v3/streams/?username=";
        let user = req.url.strip_prefix(prefix).unwrap_or_default().to_owned();
        let body = format!(
            r#"[{{"type":"live","thumbnail_url":"https://thumb/{user}.jpeg","viewer_count":12,
               "createdAt":"2023-11-14T22:13:20Z","user":{{"display_name":"{user}",
               "title":"live now","username":"{user}","profile_logo_url":"https://logo/{user}.png"}}}}]"#
        );
        Box::pin(async move {
            Ok(HttpResponse {
                status: 200,
                headers: Vec::new(),
                body: body.into_bytes(),
            })
        })
    }
}

fn angelthump(n: &mut Network) {
    let d = n
        .server_config
        .as_mut()
        .unwrap()
        .directory
        .get_or_insert_default();
    d.integrations = Some(Integrations {
        angelthump: Some(AngelThump { enable: true }),
        ..Default::default()
    });
}

/// A member's embed listing of a service the network enables shows the
/// snippet the server loads through its fetcher, again every load
/// interval; a service the network doesn't enable is refused.
#[test]
fn a_members_embed_shows_the_snippet_the_server_loads() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let requests = Rc::new(Cell::new(0));
        owner.directory.set_fetch(Fetch::new(FakeAngelThump {
            requests: requests.clone(),
        }));
        let (key, _pipes) = owned_network_with(&owner, &[&alice], 12, angelthump).await;
        let key = key.public;
        let client = alice.directory.client_config(&key).unwrap();
        let i = client.integrations.unwrap();
        assert!(i.angelthump && !i.twitch && !i.youtube);

        let twitch = embed::listing(EmbedService::TwitchStream, "destiny");
        assert_eq!(
            alice.directory.publish(&key, twitch).await,
            Err(CallFailure::Refused(DirectoryError::UnsupportedEmbed))
        );
        let listing = embed::listing(EmbedService::AngelThump, "psrngafk");
        let id = alice.directory.publish(&key, listing).await.unwrap();
        let snippet = |n: &Node| {
            n.directory
                .listings(12)
                .into_iter()
                .find(|l| l.id == id)
                .and_then(|l| l.snippet)
        };
        wait_for(10, "the loaded snippet", || {
            snippet(&alice).is_some_and(|s| s.title == "live now")
        })
        .await;
        let s = snippet(&alice).unwrap();
        assert!(s.live);
        assert_eq!((s.user_count, s.channel_name.as_str()), (12, "psrngafk"));
        assert_eq!(s.start_time, 1_700_000_000);
        assert_eq!(requests.get(), 1);

        // the next load's live thumbnail is fetched again
        sleep_secs(62).await;
        assert_eq!(requests.get(), 2);
        let m = owner.directory.metrics(&key).unwrap();
        assert_eq!((m.embed_batches, m.embed_batch_errors), (2, 0));
        let later = snippet(&alice).unwrap();
        assert_eq!(later.title, s.title);
        assert_ne!(later.thumbnail, s.thumbnail);
    });
}
