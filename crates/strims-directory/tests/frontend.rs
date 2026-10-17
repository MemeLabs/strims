// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The directory frontend on fake nodes (see `support`), mostly through a
//! UI's RPC connection: calls by network key, reads of the views, and
//! watches of listings, listing users and asset bundles.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use std::collections::BTreeSet;

use futures_util::{FutureExt, Stream, StreamExt};
use strims_base::rng::det_rng;
use strims_dao::schema::directory::DirectoryUserRecords;
use strims_dao::schema::network::{Networks, network_scoped_key};
use strims_dao::{Recv, Table};
use strims_directory::{DirectoryFrontend, bind_frontend};
use strims_network::membership::{network_from_invitation_v0, new_invitation_v0, new_network};
use strims_proto::network::v1::Network;
use strims_proto::network::v1::directory::frontend_watch_listing_users_response::UserEventType;
use strims_proto::network::v1::directory::frontend_watch_listings_response::event::Event as Body;
use strims_proto::network::v1::directory::listing::{Chat, Content, Embed, Service};
use strims_proto::network::v1::directory::listing_query::Query;
use strims_proto::network::v1::directory::{
    DirectoryFrontendClient, DirectoryFrontendService, FrontendGetListingRequest,
    FrontendGetListingsRequest, FrontendGetUsersRequest, FrontendJoinRequest,
    FrontendModerateListingRequest, FrontendModerateUserRequest, FrontendPartRequest,
    FrontendPublishRequest, FrontendTestRequest, FrontendUnpublishRequest,
    FrontendWatchAssetBundlesRequest, FrontendWatchListingUsersRequest,
    FrontendWatchListingUsersResponse, FrontendWatchListingsRequest, FrontendWatchListingsResponse,
    Listing, ListingContentType, ListingModeration, ListingQuery, UserModeration,
};
use strims_proto::service::{CallError, Context, Error as RpcError};
use strims_rpc::{Client, Router};
use support::*;

type Ui = DirectoryFrontendClient<Client>;

/// A UI connected to `n`'s directory frontend.
fn ui(n: &Node) -> Ui {
    let router = Router::new();
    bind_frontend(&router, &n.directory);
    DirectoryFrontendClient::new(frontend_client(router))
}

fn chat(n: u8) -> Listing {
    Listing {
        content: Some(Content::Chat(Chat {
            key: vec![n; 32],
            name: format!("chat {n}"),
        })),
    }
}

fn service(n: u8) -> Listing {
    Listing {
        content: Some(Content::Service(Service {
            r#type: format!("service {n}"),
            swarm_uri: String::new(),
        })),
    }
}

fn by_id(id: u64) -> Option<ListingQuery> {
    Some(ListingQuery {
        query: Some(Query::Id(id)),
    })
}

fn by_listing(l: Listing) -> Option<ListingQuery> {
    Some(ListingQuery {
        query: Some(Query::Listing(l)),
    })
}

fn types(ts: &[ListingContentType]) -> Vec<i32> {
    ts.iter().map(|t| i32::from(*t)).collect()
}

/// The message of a call's remote error.
fn refusal<T: std::fmt::Debug>(r: Result<T, CallError>) -> String {
    match r {
        Err(CallError::Remote(e)) => e.message,
        r => panic!("not refused: {r:?}"),
    }
}

/// The message of the remote error a streaming call starts with.
async fn stream_refusal<S, T>(r: Result<S, CallError>) -> String
where
    S: Stream<Item = Result<T, CallError>> + Unpin,
    T: std::fmt::Debug,
{
    let mut s = r.unwrap();
    match s.next().await {
        Some(Err(CallError::Remote(e))) => e.message,
        r => panic!("not refused: {r:?}"),
    }
}

/// The items a stream has ready.
fn ready<S: Stream + Unpin>(s: &mut S) -> Vec<S::Item> {
    let mut out = Vec::new();
    while let Some(Some(r)) = s.next().now_or_never() {
        out.push(r);
    }
    out
}

/// The events of a listings watch's ready responses.
fn listing_events<S, E>(s: &mut S) -> Vec<Body>
where
    S: Stream<Item = Result<FrontendWatchListingsResponse, E>> + Unpin,
    E: std::fmt::Debug,
{
    ready(s)
        .into_iter()
        .flat_map(|r| r.unwrap().events)
        .map(|e| e.event.unwrap())
        .collect()
}

/// A `Change` event's network id and listing ids.
fn changed(b: &Body) -> Option<(u64, Vec<u64>)> {
    match b {
        Body::Change(c) => {
            let ls = c.listings.as_ref().unwrap();
            let ids = ls.listings.iter().map(|l| l.id).collect();
            Some((ls.network.as_ref().unwrap().id, ids))
        }
        _ => None,
    }
}

/// The runs of a listing users watch's ready responses: their type and
/// aliases.
fn user_runs<S, E>(s: &mut S) -> Vec<(UserEventType, Vec<String>)>
where
    S: Stream<Item = Result<FrontendWatchListingUsersResponse, E>> + Unpin,
    E: std::fmt::Debug,
{
    ready(s)
        .into_iter()
        .map(|r| {
            let r = r.unwrap();
            let ty = UserEventType::try_from(r.r#type).unwrap();
            (ty, r.users.into_iter().map(|u| u.alias).collect())
        })
        .collect()
}

fn run_of(ty: UserEventType, aliases: &[&str]) -> (UserEventType, Vec<String>) {
    (ty, aliases.iter().map(|a| (*a).to_owned()).collect())
}

/// Network `id`, created by `owner` and joined by `members` over the links
/// they already have.
async fn another_network(owner: &Node, members: &[&Node], id: u64) -> Vec<u8> {
    let mut rng = det_rng(id);
    let owned = new_network(id, "other", None, &owner.profile, "", T0, &mut rng).unwrap();
    let owner_cert = owned.certificate.clone().unwrap();
    let key = owned.server_config.as_ref().unwrap().key.clone().unwrap();
    owner.write_network(owned).await;
    for m in members {
        let invitation = new_invitation_v0(&owner.profile, &owner_cert, T0, &mut rng).unwrap();
        let joined =
            network_from_invitation_v0(id, &invitation, &m.profile, "", T0, &mut rng).unwrap();
        m.write_network(joined).await;
    }
    settle().await;
    key.public
}

async fn delete_network(n: &Node, id: u64) {
    let mut tx = n.store.write(T0).await.unwrap();
    Networks::delete(&mut tx, id).await.unwrap();
    tx.commit().await.unwrap();
}

/// Publishing, joining, parting and unpublishing reach the directory of
/// the network a key names; the server's refusals come as its messages,
/// and an unknown key is refused.
#[test]
fn calls_reach_the_directory_of_a_networks_key() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let bob = node_in(&world, 3).await;
        let (key, _pipes) = owned_network(&owner, &[&alice, &bob], 10).await;
        let key = key.public;
        let (ua, ub) = (ui(&alice), ui(&bob));

        let id = ua
            .publish(&FrontendPublishRequest {
                network_key: key.clone(),
                listing: Some(chat(1)),
            })
            .await
            .unwrap()
            .id;
        let joined = ub
            .join(&FrontendJoinRequest {
                network_key: key.clone(),
                query: by_listing(chat(1)),
            })
            .await
            .unwrap()
            .id;
        assert_eq!(joined, id);
        sleep_secs(3).await;
        let get = FrontendGetListingRequest {
            query: by_id(id),
            network_key: key.clone(),
        };
        let got = ub.get_listing(&get).await.unwrap();
        assert_eq!(
            (got.id, got.listing, got.user_count),
            (id, Some(chat(1)), 2)
        );
        let by_query = FrontendGetListingRequest {
            query: by_listing(chat(1)),
            network_key: key.clone(),
        };
        assert_eq!(ub.get_listing(&by_query).await.unwrap().id, id);

        ub.part(&FrontendPartRequest {
            network_key: key.clone(),
            id,
        })
        .await
        .unwrap();
        ua.unpublish(&FrontendUnpublishRequest {
            network_key: key.clone(),
            id,
        })
        .await
        .unwrap();
        sleep_secs(3).await;
        assert_eq!(refusal(ub.get_listing(&get).await), "listing not found");
        assert!(alice.directory.listings(10).is_empty());

        let embed = Listing {
            content: Some(Content::Embed(Embed::default())),
        };
        let refused = ua
            .publish(&FrontendPublishRequest {
                network_key: key.clone(),
                listing: Some(embed),
            })
            .await;
        assert_eq!(refusal(refused), "unsupported embed service");
        let unknown = ua
            .publish(&FrontendPublishRequest {
                network_key: vec![9; 32],
                listing: Some(chat(2)),
            })
            .await;
        assert_eq!(refusal(unknown), "network not found");
        let unknown = ua
            .get_listing(&FrontendGetListingRequest {
                query: by_id(id),
                network_key: vec![9; 32],
            })
            .await;
        assert_eq!(refusal(unknown), "network not found");
    });
}

/// The owner makes a member a moderator by alias, and the moderator bans a
/// listing; an alias the CA never issued, and a member without the right,
/// are refused.
#[test]
fn moderating_users_by_alias_and_listings() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let bob = node_in(&world, 3).await;
        let (key, _pipes) = owned_network(&owner, &[&alice, &bob], 11).await;
        let key = key.public;
        let (uo, ua, ub) = (ui(&owner), ui(&alice), ui(&bob));
        let alias = alice.read_network(11).await.certificate.unwrap().subject;
        assert_eq!(alias, "node-2");

        uo.moderate_user(&FrontendModerateUserRequest {
            network_key: key.clone(),
            alias,
            moderation: Some(UserModeration {
                is_moderator: Some(true),
                ..Default::default()
            }),
        })
        .await
        .unwrap();
        let rec = {
            let mut tx = owner.store.read().await.unwrap();
            let k = network_scoped_key(11, &alice.profile_key().public);
            DirectoryUserRecords::index_get(&mut tx, &DirectoryUserRecords::BY_PEER_KEY, &k)
                .await
                .unwrap()
                .unwrap()
        };
        assert_eq!(rec.moderation.unwrap().is_moderator, Some(true));

        let id = bob.directory.publish(&key, chat(1)).await.unwrap();
        let ban = FrontendModerateListingRequest {
            network_key: key.clone(),
            id,
            moderation: Some(ListingModeration {
                is_banned: Some(true),
                ..Default::default()
            }),
        };
        assert_eq!(
            refusal(ub.moderate_listing(&ban).await),
            "permission denied"
        );
        ua.moderate_listing(&ban).await.unwrap();
        sleep_secs(2).await;
        assert!(bob.directory.listings(11).is_empty());

        let nobody = uo
            .moderate_user(&FrontendModerateUserRequest {
                network_key: key.clone(),
                alias: "nobody".into(),
                moderation: None,
            })
            .await;
        assert_eq!(refusal(nobody), "not found");
    });
}

/// The users of every view, merged by peer key, with the networks each
/// alias is used on, and the networks by id.
#[test]
fn users_merge_by_peer_key_and_alias() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (first, _pipes) = owned_network(&owner, &[&alice], 12).await;
        let first = first.public;
        let second = another_network(&owner, &[&alice], 13).await;
        alice.directory.publish(&first, chat(1)).await.unwrap();
        alice.directory.publish(&second, chat(1)).await.unwrap();
        sleep_secs(3).await;
        let ua = ui(&alice);

        let got = ua.get_users(&FrontendGetUsersRequest {}).await.unwrap();
        assert_eq!(got.networks.keys().copied().collect::<Vec<_>>(), [12, 13]);
        assert_eq!(got.networks[&12].key, first);
        assert_eq!(got.networks[&12].name, "net");
        assert_eq!(got.networks[&13].name, "other");
        let users: Vec<_> = got
            .users
            .iter()
            .filter(|u| u.peer_key == alice.profile_key().public)
            .collect();
        assert_eq!(users.len(), 1, "{got:?}");
        let aliases: Vec<(&str, &[u64])> = users[0]
            .aliases
            .iter()
            .map(|a| (a.alias.as_str(), a.network_ids.as_slice()))
            .collect();
        assert_eq!(aliases, [("node-2", &[12, 13][..])]);

        alice
            .change_network(13, |n| n.alias = "alicia".into())
            .await;
        settle().await;
        let got = ua.get_users(&FrontendGetUsersRequest {}).await.unwrap();
        let user = got
            .users
            .iter()
            .find(|u| u.peer_key == alice.profile_key().public)
            .unwrap();
        let aliases: Vec<(&str, &[u64])> = user
            .aliases
            .iter()
            .map(|a| (a.alias.as_str(), a.network_ids.as_slice()))
            .collect();
        assert_eq!(aliases, [("node-2", &[12][..]), ("alicia", &[13][..])]);
    });
}

/// Listings by network and content type: every profile network, or those
/// the request names, each with the view's listings of the types asked
/// for.
#[test]
fn listings_by_network_and_content_type() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (key, _pipes) = owned_network(&owner, &[&alice], 14).await;
        let key = key.public;
        let chat_id = alice.directory.publish(&key, chat(1)).await.unwrap();
        let service_id = alice.directory.publish(&key, service(2)).await.unwrap();
        sleep_secs(3).await;
        let ua = ui(&alice);
        let get = |network_keys: Vec<Vec<u8>>, content_types: Vec<i32>| {
            let ua = ua.clone();
            async move {
                ua.get_listings(&FrontendGetListingsRequest {
                    content_types,
                    network_keys,
                })
                .await
            }
        };

        let all = get(vec![], vec![]).await.unwrap().listings;
        assert_eq!(all.len(), 1);
        let network = all[0].network.as_ref().unwrap();
        assert_eq!(
            (network.id, &network.key, network.name.as_str()),
            (14, &key, "net")
        );
        let ids: BTreeSet<u64> = all[0].listings.iter().map(|l| l.id).collect();
        assert_eq!(ids, BTreeSet::from([chat_id, service_id]));
        let item = all[0].listings.iter().find(|l| l.id == chat_id).unwrap();
        assert_eq!((&item.listing, item.user_count), (&Some(chat(1)), 1));

        let chats = get(vec![key.clone()], types(&[ListingContentType::Chat]))
            .await
            .unwrap()
            .listings;
        let ids: Vec<u64> = chats[0].listings.iter().map(|l| l.id).collect();
        assert_eq!(ids, [chat_id]);
        let media = get(vec![], types(&[ListingContentType::Media]))
            .await
            .unwrap()
            .listings;
        assert_eq!(media.len(), 1);
        assert!(media[0].listings.is_empty());
        let unknown = get(vec![key.clone(), vec![9; 32]], vec![]).await;
        assert_eq!(refusal(unknown), "network not found");
    });
}

/// A listings watch starts with a `Change` per network, then sends its
/// changes, filtered by content type or listing id.
#[test]
fn watching_listings() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let bob = node_in(&world, 3).await;
        let (key, _pipes) = owned_network(&owner, &[&alice, &bob], 15).await;
        let key = key.public;
        let ua = ui(&alice);
        let watch = |content_types: Vec<i32>, network_keys: Vec<Vec<u8>>, listing_id: u64| {
            ua.watch_listings(&FrontendWatchListingsRequest {
                content_types,
                network_keys,
                listing_id,
            })
        };
        let mut all = watch(vec![], vec![], 0).await.unwrap();
        let mut services = watch(types(&[ListingContentType::Service]), vec![key.clone()], 0)
            .await
            .unwrap();
        sleep_secs(1).await;
        let first = listing_events(&mut all);
        assert_eq!(first.len(), 1, "{first:?}");
        let Body::Change(c) = &first[0] else {
            panic!("{first:?}");
        };
        let ls = c.listings.as_ref().unwrap();
        let network = ls.network.as_ref().unwrap();
        assert_eq!(
            (network.id, &network.key, network.name.as_str()),
            (15, &key, "net")
        );
        assert!(ls.listings.is_empty());
        let first = listing_events(&mut services);
        assert_eq!(
            first.iter().map(changed).collect::<Vec<_>>(),
            [Some((15, vec![]))]
        );

        let id = bob.directory.publish(&key, chat(1)).await.unwrap();
        sleep_secs(3).await;
        let got = listing_events(&mut all);
        assert!(
            got.iter().any(|b| changed(b) == Some((15, vec![id]))),
            "{got:?}"
        );
        assert!(listing_events(&mut services).is_empty());

        let mut one = watch(vec![], vec![key.clone()], id).await.unwrap();
        sleep_secs(1).await;
        let got = listing_events(&mut one);
        assert_eq!(
            got.iter().map(changed).collect::<Vec<_>>(),
            [Some((15, vec![id]))]
        );

        alice
            .directory
            .join(&key, by_id(id).unwrap())
            .await
            .unwrap();
        sleep_secs(3).await;
        let got = listing_events(&mut all);
        assert!(
            got.iter().any(|b| matches!(b,
                Body::UserCountChange(c) if (c.network_id, c.listing_id, c.user_count) == (15, id, 2))),
            "{got:?}"
        );

        let svc = bob.directory.publish(&key, service(2)).await.unwrap();
        sleep_secs(3).await;
        let got = listing_events(&mut services);
        assert!(
            got.iter().any(|b| changed(b) == Some((15, vec![svc]))),
            "{got:?}"
        );
        assert!(
            got.iter()
                .all(|b| changed(b).is_none_or(|(_, ids)| ids == [svc]))
        );

        alice.directory.part(&key, id).await.unwrap();
        bob.directory.unpublish(&key, id).await.unwrap();
        sleep_secs(3).await;
        let got = listing_events(&mut all);
        assert!(
            got.iter().any(|b| matches!(b,
                Body::Unpublish(u) if (u.network_id, u.listing_id) == (15, id))),
            "{got:?}"
        );
        let got = listing_events(&mut one);
        assert!(
            got.iter().any(|b| matches!(b,
                Body::Unpublish(u) if (u.network_id, u.listing_id) == (15, id))),
            "{got:?}"
        );

        drop((all, services, one));
        sleep_secs(1).await;
        let listed = ua
            .get_listings(&FrontendGetListingsRequest::default())
            .await
            .unwrap();
        assert_eq!(listed.listings[0].listings.len(), 1);
        let unknown = watch(vec![], vec![vec![9; 32]], 0).await;
        assert_eq!(stream_refusal(unknown).await, "network not found");
    });
}

/// A watch of every network takes a network whose directory starts after
/// it, and unpublishes a stopped network's listings.
#[test]
fn a_network_that_starts_later_joins_a_watch() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let ua = ui(&alice);
        let mut all = ua
            .watch_listings(&FrontendWatchListingsRequest::default())
            .await
            .unwrap();
        sleep_secs(1).await;
        assert!(listing_events(&mut all).is_empty());

        let (key, _pipes) = owned_network(&owner, &[&alice], 16).await;
        let key = key.public;
        let got = listing_events(&mut all);
        assert!(
            got.iter().any(|b| changed(b).is_some_and(|(n, _)| n == 16)),
            "{got:?}"
        );
        let id = alice.directory.publish(&key, chat(1)).await.unwrap();
        sleep_secs(3).await;
        let got = listing_events(&mut all);
        assert!(
            got.iter().any(|b| changed(b) == Some((16, vec![id]))),
            "{got:?}"
        );

        delete_network(&alice, 16).await;
        sleep_secs(3).await;
        let got = listing_events(&mut all);
        assert!(
            got.iter().any(|b| matches!(b,
                Body::Unpublish(u) if (u.network_id, u.listing_id) == (16, id))),
            "{got:?}"
        );
    });
}

/// A listing users watch starts with the listing's users, though the view
/// holds the listing only later, then sends runs of joins, renames and
/// parts; the users part when the listing goes, and join when it's back.
#[test]
fn watching_listing_users() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let bob = node_in(&world, 3).await;
        let (key, _pipes) = owned_network(&owner, &[&alice, &bob], 18).await;
        let key = key.public;
        let ua = ui(&alice);
        let mut users = ua
            .watch_listing_users(&FrontendWatchListingUsersRequest {
                network_key: key.clone(),
                query: by_listing(chat(1)),
            })
            .await
            .unwrap();
        sleep_secs(1).await;
        assert_eq!(user_runs(&mut users), [run_of(UserEventType::Join, &[])]);

        let id = bob.directory.publish(&key, chat(1)).await.unwrap();
        sleep_secs(3).await;
        assert_eq!(
            user_runs(&mut users),
            [run_of(UserEventType::Join, &["node-3"])]
        );
        alice
            .directory
            .join(&key, by_id(id).unwrap())
            .await
            .unwrap();
        sleep_secs(3).await;
        assert_eq!(
            user_runs(&mut users),
            [run_of(UserEventType::Join, &["node-2"])]
        );

        alice
            .change_network(18, |n| n.alias = "alicia".into())
            .await;
        settle().await;
        assert_eq!(
            user_runs(&mut users),
            [run_of(UserEventType::Rename, &["alicia"])]
        );

        alice.directory.part(&key, id).await.unwrap();
        bob.directory.unpublish(&key, id).await.unwrap();
        sleep_secs(3).await;
        let got = user_runs(&mut users);
        let parted: BTreeSet<String> = got
            .iter()
            .inspect(|(ty, _)| assert_eq!(*ty, UserEventType::Part, "{got:?}"))
            .flat_map(|(_, a)| a.iter().cloned())
            .collect();
        assert_eq!(parted, BTreeSet::from(["alicia".into(), "node-3".into()]));

        bob.directory.publish(&key, chat(1)).await.unwrap();
        sleep_secs(3).await;
        assert_eq!(
            user_runs(&mut users),
            [run_of(UserEventType::Join, &["node-3"])]
        );

        let unknown = ua
            .watch_listing_users(&FrontendWatchListingUsersRequest {
                network_key: vec![9; 32],
                query: by_id(id),
            })
            .await;
        assert_eq!(stream_refusal(unknown).await, "network not found");
    });
}

/// An asset bundles watch sends each network's latest bundle, then its
/// changes.
#[test]
fn watching_asset_bundles() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (key, _pipes) = owned_network(&owner, &[&alice], 19).await;
        let key = key.public;
        let ua = ui(&alice);
        let mut bundles = ua
            .watch_asset_bundles(&FrontendWatchAssetBundlesRequest {})
            .await
            .unwrap();
        sleep_secs(1).await;
        let got: Vec<_> = ready(&mut bundles)
            .into_iter()
            .map(Result::unwrap)
            .collect();
        assert_eq!(got.len(), 1);
        assert_eq!((got[0].network_id, &got[0].network_key), (19, &key));
        let quota =
            |b: &strims_proto::network::v1::directory::FrontendWatchAssetBundlesResponse| {
                b.asset_bundle
                    .as_ref()
                    .unwrap()
                    .directory
                    .as_ref()
                    .unwrap()
                    .publish_quota
            };
        assert_eq!(quota(&got[0]), 10);

        owner
            .change_network(19, |n| {
                let c = n.server_config.as_mut().unwrap();
                c.directory.get_or_insert_default().publish_quota = 3;
            })
            .await;
        sleep_secs(3).await;
        let got: Vec<_> = ready(&mut bundles)
            .into_iter()
            .map(Result::unwrap)
            .collect();
        assert_eq!(got.len(), 1, "{got:?}");
        assert_eq!(quota(&got[0]), 3);
    });
}

/// A UI too slow for its watches' subscriptions gets the views again: the
/// listings it holds that went, the listings there are, the users that
/// parted, and the latest asset bundles.
#[test]
fn lagging_watches_get_the_views_again() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let bob = node_in(&world, 3).await;
        // a refresh every second fills every subscription that isn't read
        let (key, _pipes) = owned_network_with(&owner, &[&alice, &bob], 17, |n: &mut Network| {
            let d = n
                .server_config
                .as_mut()
                .unwrap()
                .directory
                .get_or_insert_default();
            d.refresh_interval = 1;
        })
        .await;
        let key = key.public;
        let mut ids = Vec::new();
        for i in 1..=4 {
            ids.push(alice.directory.publish(&key, chat(i)).await.unwrap());
        }
        bob.directory
            .join(&key, by_id(ids[0]).unwrap())
            .await
            .unwrap();
        sleep_secs(3).await;

        let f = DirectoryFrontend::new(alice.directory.clone());
        let ctx = Context::default();
        let mut listings = f
            .watch_listings(&ctx, FrontendWatchListingsRequest::default())
            .await
            .unwrap();
        let mut users = f
            .watch_listing_users(
                &ctx,
                FrontendWatchListingUsersRequest {
                    network_key: key.clone(),
                    query: by_id(ids[0]),
                },
            )
            .await
            .unwrap();
        let mut bundles = f
            .watch_asset_bundles(&ctx, FrontendWatchAssetBundlesRequest {})
            .await
            .unwrap();
        let first = listing_events(&mut listings);
        let all: BTreeSet<u64> = ids.iter().copied().collect();
        assert!(
            first
                .iter()
                .filter_map(changed)
                .any(|(n, l)| n == 17 && l.iter().copied().collect::<BTreeSet<_>>() == all),
            "{first:?}"
        );
        let mut joined = user_runs(&mut users);
        joined[0].1.sort();
        assert_eq!(joined, [run_of(UserEventType::Join, &["node-2", "node-3"])]);
        assert_eq!(ready(&mut bundles).len(), 1);

        let mut probe = alice.directory.subscribe();
        alice.directory.unpublish(&key, ids[3]).await.unwrap();
        bob.directory.part(&key, ids[0]).await.unwrap();
        sleep_secs(120).await;
        assert_eq!(probe.next().now_or_never(), Some(Some(Recv::Lagged)));

        let got = listing_events(&mut listings);
        assert_eq!(got.len(), 2, "{got:?}");
        assert!(
            matches!(&got[0], Body::Unpublish(u) if (u.network_id, u.listing_id) == (17, ids[3]))
        );
        assert_eq!(changed(&got[1]), Some((17, ids[..3].to_vec())));
        assert_eq!(
            user_runs(&mut users),
            [run_of(UserEventType::Part, &["node-3"])]
        );
        let got: Vec<_> = ready(&mut bundles)
            .into_iter()
            .map(Result::unwrap)
            .collect();
        assert_eq!(got.len(), 1);
        assert_eq!((got[0].network_id, &got[0].network_key), (17, &key));
    });
}

/// `Test` has no implementation.
#[test]
fn test_is_unimplemented() {
    run(async {
        let world = World::default();
        let alice = node_in(&world, 2).await;
        let got = ui(&alice)
            .test(&FrontendTestRequest {
                network_key: vec![1; 32],
            })
            .await;
        assert_eq!(got, Err(CallError::Remote(RpcError::unimplemented())));
    });
}
