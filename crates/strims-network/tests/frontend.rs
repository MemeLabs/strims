// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The network frontend on fake nodes (see `support`): creating and
//! joining networks, watching them, and administering a network's CA.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use data_encoding::BASE64_NOPAD;
use futures_util::{FutureExt, StreamExt};
use strims_network::NetworkFrontend;
use strims_proto::network::v1::WatchNetworksResponse;
use strims_proto::network::v1::create_network_from_invitation_request::Invitation as InvitationField;
use strims_proto::network::v1::network_event::Body;
use strims_proto::network::v1::{
    CreateInvitationRequest, CreateNetworkFromInvitationRequest, CreateServerRequest,
    DeleteNetworkRequest, DeletePeerRequest, GetUiConfigRequest, GrantPeerInvitationRequest,
    ListAliasReservationsRequest, ListNetworksRequest, ListPeersRequest, NetworkFrontendService,
    ResetAliasReservationCooldownRequest, ServerConfig, TogglePeerBanRequest, UpdateAliasRequest,
    UpdateDisplayOrderRequest, UpdateServerConfigRequest, WatchNetworksRequest,
};
use strims_proto::service::{Context, ServerStream};
use support::*;

type Frontend = NetworkFrontend<strims_kv::MemoryKv, FakeHost, TokioTimer>;

fn frontend(n: &Node) -> Frontend {
    NetworkFrontend::new(n.control.clone())
}

fn ctx() -> Context {
    Context::default()
}

/// The events a watch has ready.
fn drain(w: &mut ServerStream<WatchNetworksResponse>) -> Vec<Body> {
    let mut out = Vec::new();
    while let Some(Some(r)) = w.next().now_or_never() {
        out.push(r.unwrap().event.unwrap().body.unwrap());
    }
    out
}

/// An owner creates a network and invites a profile, which joins from the
/// invitation's text; once linked, they bind on it.
#[test]
fn a_network_created_and_joined_through_the_frontend_binds() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let invitee = node_in(&world, 2).await;
        let (fo, fi) = (frontend(&owner), frontend(&invitee));

        let created = fo
            .create_server(
                &ctx(),
                CreateServerRequest {
                    name: "net".into(),
                    alias: "boss".into(),
                    ..Default::default()
                },
            )
            .await
            .unwrap()
            .network
            .unwrap();
        assert_eq!(created.alias, "boss");
        assert_eq!(created.certificate.as_ref().unwrap().subject, "boss");
        let key = created.server_config.as_ref().unwrap().key.clone().unwrap();

        let invitation = fo
            .create_invitation(
                &ctx(),
                CreateInvitationRequest {
                    network_id: created.id,
                    bootstrap_client_id: 0,
                },
            )
            .await
            .unwrap()
            .invitation
            .unwrap();
        let text = BASE64_NOPAD.encode(&prost::Message::encode_to_vec(&invitation));
        let joined = fi
            .create_network_from_invitation(
                &ctx(),
                CreateNetworkFromInvitationRequest {
                    alias: String::new(),
                    invitation: Some(InvitationField::InvitationB64(text)),
                },
            )
            .await
            .unwrap()
            .network
            .unwrap();
        assert!(!strims_crypto::membership_trusted(
            joined.certificate.as_ref().unwrap(),
            T0
        ));

        let _pipes = link(&owner, &invitee);
        settle().await;
        assert_bound(&owner, &invitee, &key);

        let listed = fi
            .list(&ctx(), ListNetworksRequest {})
            .await
            .unwrap()
            .networks;
        assert_eq!(listed.len(), 1);
        assert!(strims_crypto::membership_trusted(
            listed[0].certificate.as_ref().unwrap(),
            T0
        ));

        // the server config's key never changes
        let bad = fo
            .update_server_config(
                &ctx(),
                UpdateServerConfigRequest {
                    network_id: created.id,
                    server_config: Some(ServerConfig {
                        name: "renamed".into(),
                        ..Default::default()
                    }),
                },
            )
            .await;
        assert!(bad.is_err());
    });
}

/// A watch starts with the running networks, then follows peer counts,
/// renames, the UI order and deletions.
#[test]
fn watching_networks() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 3).await;
        let b = node_in(&world, 4).await;
        let (fa, fb) = (frontend(&a), frontend(&b));
        let created = fa
            .create_server(
                &ctx(),
                CreateServerRequest {
                    name: "net".into(),
                    ..Default::default()
                },
            )
            .await
            .unwrap()
            .network
            .unwrap();
        settle().await;
        let mut w = fa.watch(&ctx(), WatchNetworksRequest {}).await.unwrap();
        let first = drain(&mut w);
        assert!(
            matches!(&first[..], [Body::NetworkStart(s)] if s.network.as_ref().unwrap().id == created.id && s.peer_count == 0),
            "{first:?}"
        );

        let invitation = fa
            .create_invitation(
                &ctx(),
                CreateInvitationRequest {
                    network_id: created.id,
                    bootstrap_client_id: 0,
                },
            )
            .await
            .unwrap()
            .invitation
            .unwrap();
        fb.create_network_from_invitation(
            &ctx(),
            CreateNetworkFromInvitationRequest {
                alias: String::new(),
                invitation: Some(InvitationField::InvitationBytes(
                    prost::Message::encode_to_vec(&invitation),
                )),
            },
        )
        .await
        .unwrap();
        let _pipes = link(&a, &b);
        settle().await;
        fa.update_alias(
            &ctx(),
            UpdateAliasRequest {
                id: created.id,
                alias: "renamed".into(),
            },
        )
        .await
        .unwrap();
        fa.update_display_order(
            &ctx(),
            UpdateDisplayOrderRequest {
                network_ids: vec![created.id],
            },
        )
        .await
        .unwrap();
        settle().await;
        let events = drain(&mut w);
        assert!(events.iter().any(|e| matches!(
            e,
            Body::NetworkPeerCountUpdate(u) if u.network_id == created.id && u.peer_count == 1
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            Body::NetworkUpdate(n) if n.certificate.as_ref().unwrap().subject == "renamed"
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            Body::UiConfigUpdate(c) if c.network_display_order == [created.id]
        )));
        let ui = fa
            .get_ui_config(&ctx(), GetUiConfigRequest {})
            .await
            .unwrap()
            .config
            .unwrap();
        assert_eq!(ui.network_display_order, [created.id]);

        fa.delete(&ctx(), DeleteNetworkRequest { id: created.id })
            .await
            .unwrap();
        settle().await;
        let events = drain(&mut w);
        assert!(events.iter().any(|e| matches!(
            e,
            Body::NetworkStop(s) if s.network_id == created.id
        )));
    });
}

/// The owner lists and administers its network's peers and aliases.
#[test]
fn administering_a_networks_peers() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 5).await;
        let invitee = node_in(&world, 6).await;
        let (fo, fi) = (frontend(&owner), frontend(&invitee));
        let created = fo
            .create_server(
                &ctx(),
                CreateServerRequest {
                    name: "net".into(),
                    ..Default::default()
                },
            )
            .await
            .unwrap()
            .network
            .unwrap();
        let invitation = fo
            .create_invitation(
                &ctx(),
                CreateInvitationRequest {
                    network_id: created.id,
                    bootstrap_client_id: 0,
                },
            )
            .await
            .unwrap()
            .invitation
            .unwrap();
        fi.create_network_from_invitation(
            &ctx(),
            CreateNetworkFromInvitationRequest {
                alias: "guest".into(),
                invitation: Some(InvitationField::InvitationBytes(
                    prost::Message::encode_to_vec(&invitation),
                )),
            },
        )
        .await
        .unwrap();
        let _pipes = link(&owner, &invitee);
        settle().await;

        let peers = fo
            .list_peers(
                &ctx(),
                ListPeersRequest {
                    network_id: created.id,
                },
            )
            .await
            .unwrap()
            .peers;
        assert_eq!(peers.len(), 2);
        let guest = peers.iter().find(|p| p.alias == "guest").unwrap().clone();
        let granted = fo
            .grant_peer_invitation(
                &ctx(),
                GrantPeerInvitationRequest {
                    id: guest.id,
                    count: 2,
                },
            )
            .await
            .unwrap()
            .peer
            .unwrap();
        assert_eq!(granted.invite_quota, guest.invite_quota + 2);
        let banned = fo
            .toggle_peer_ban(
                &ctx(),
                TogglePeerBanRequest {
                    id: guest.id,
                    value: true,
                },
            )
            .await
            .unwrap()
            .peer
            .unwrap();
        assert!(banned.is_banned);

        let reservations = |f: Frontend| async move {
            f.list_alias_reservations(
                &ctx(),
                ListAliasReservationsRequest {
                    network_id: created.id,
                },
            )
            .await
            .unwrap()
            .alias_reservations
        };
        let held = reservations(frontend(&owner)).await;
        let guest_alias = held.iter().find(|r| r.alias == "guest").unwrap().clone();
        assert!(
            fo.reset_alias_reservation_cooldown(
                &ctx(),
                ResetAliasReservationCooldownRequest { id: guest_alias.id }
            )
            .await
            .is_err()
        );

        fo.delete_peer(&ctx(), DeletePeerRequest { id: guest.id })
            .await
            .unwrap();
        let after = reservations(frontend(&owner)).await;
        let released = after.iter().find(|r| r.alias == "guest").unwrap();
        assert!(released.peer_key.is_empty());
        assert!(released.reserved_until > T0.unix());
        fo.reset_alias_reservation_cooldown(
            &ctx(),
            ResetAliasReservationCooldownRequest { id: released.id },
        )
        .await
        .unwrap();
        let reset = reservations(frontend(&owner)).await;
        assert_eq!(
            reset
                .iter()
                .find(|r| r.alias == "guest")
                .unwrap()
                .reserved_until,
            0
        );
    });
}
