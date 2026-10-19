// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A recorded profile store (`golden/store.hex`) that pins the store layout:
//! its sign-in and session records open, its records read back, every key
//! is the one this crate derives, the compressed delta matches, its
//! certificates verify at the recording time, its stream keys name their
//! channels, and its swarm cache reads back by swarm id and salt.
//!
//! [`record`] writes the store through this crate's API, from seeded rngs
//! at a fixed time, and renders it as the file's lines: `now`, `auth`,
//! `session`, `video_stream_key` and `swarm_cache` for what the scenario
//! made, then every `kv` row (table, key, value), every `record` as this
//! crate reads it back (namespace, id, encoding), and the compressed
//! `delta` a replica at `delta_remote` lacks. Fields are hex, `=` when
//! empty. A plain run checks that the recorder reproduces the file byte for
//! byte; `STRIMS_GOLDEN=update` records it again (see `strims-golden`).

// Tests panic on failure by design.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::sync::OnceLock;

use data_encoding::BASE32_NOPAD;
use futures_util::FutureExt;
use prost::Message;
use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_crypto::{
    CertificateChain, DEFAULT_CERT_TTL, Issuer, certificate_body, certificate_parent, generate_key,
    network_certificate, new_certificate_request, new_self_signed_certificate,
    sign_certificate_request, sign_certificate_request_with_network, verify_certificate,
};
use strims_dao::auth::{self, ProfileKey, SessionToken};
use strims_dao::key::{Namespace, index_entry_key, parse_base36};
use strims_dao::replication::compressed_delta;
use strims_dao::schema::autoseed::{AutoseedConfig, AutoseedRules, new_rule};
use strims_dao::schema::chat::{
    ChatEmotes, ChatModifiers, ChatProfiles, ChatServerIcons, ChatServers, ChatTags, ChatUiConfig,
    ChatUiConfigHighlights, ChatUiConfigIgnores, ChatUiConfigTags, WhisperRecords, WhisperThreads,
    profile_peer_key,
};
use strims_dao::schema::directory::{
    DirectoryListingRecords, DirectoryUserRecords, listing_key, new_listing_record, new_user_record,
};
use strims_dao::schema::network::{
    AliasReservations, BootstrapClients, BootstrapConfig, CertificateLogs, NetworkPeers,
    NetworkUiConfig, Networks, RESERVED_FOREVER, network_key, network_scoped_key,
    new_alias_reservation, new_certificate_log, new_network_peer,
};
use strims_dao::schema::notification::{Notifications, network_subject, new_notification};
use strims_dao::schema::profile::{Devices, Profile, ProfileIds};
use strims_dao::schema::swarm::{
    SwarmCacheMetas, SwarmCaches, cache_checksum, cache_key, get_cache, set_cache,
};
use strims_dao::schema::system::{Checkpoints, EventLogs, StoreVersions};
use strims_dao::schema::video::{
    HlsConfig, IngressConfig, VideoChannels, channel_by_stream_key, channel_network_key,
    format_stream_key, new_channel, owner_key,
};
use strims_dao::schema::vnic::VnicConfig;
use strims_dao::seal::StorageKey;
use strims_dao::{
    ProfileStore, ReadTx, ReplicatedStore, Singleton, Table, WriteTx, lease, ref_key,
};
use strims_kv::{Dump, MemoryKv};
use strims_proto::autoseed::v1 as autoseed;
use strims_proto::chat::v1::{self as chat, emote_effect};
use strims_proto::dao::v1::{Mutex, SecondaryIndexKey, VersionVector};
use strims_proto::network::v1::bootstrap::{
    self, BootstrapClient, BootstrapClientWebSocketOptions, bootstrap_client::ClientOptions,
};
use strims_proto::network::v1::ca::CertificateLog;
use strims_proto::network::v1::directory::listing::{self, Content};
use strims_proto::network::v1::directory::server_config::Integrations;
use strims_proto::network::v1::directory::server_config::integrations::Swarm;
use strims_proto::network::v1::directory::{
    self as directory, Listing, ListingModeration, ListingSnippet, UserModeration,
};
use strims_proto::network::v1::{Network, ServerConfig, UiConfig};
use strims_proto::notification::v1::notification::Status;
use strims_proto::profile::v1::Device;
use strims_proto::replication::v1::{Event, EventLog};
use strims_proto::types::cache::{Epoch, Integrity, MerkleIntegrity};
use strims_proto::types::{Cache, Certificate, CertificateBody, Image, ImageType, KeyUsage};
use strims_proto::video::v1::video_channel::{Local, LocalShare, Owner, RemoteShare};
use strims_proto::video::v1::{HlsEgressConfig, VideoChannel, VideoIngressConfig};
use strims_proto::vnic::v1 as vnic;

const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/golden/store.hex");

fn block<T>(f: impl Future<Output = T>) -> T {
    f.now_or_never()
        .expect("memory store operations are ready immediately")
}

/// The time the store is recorded at.
const NOW: Time = Time::from_unix_millis(1_790_000_000_000);

const PEER_SIGN: i32 = KeyUsage::Peer as i32 | KeyUsage::Sign as i32;

/// How long a released alias stays reserved.
const ALIAS_RESERVATION_COOLDOWN: Duration = Duration::from_secs(180 * 24 * 60 * 60);

/// `n` bytes counting up from `start`.
fn seq(start: u8, n: usize) -> Vec<u8> {
    (0..n).map(|i| start.wrapping_add(i as u8)).collect()
}

/// A swarm URI naming swarm `id`.
fn swarm_uri(id: &[u8]) -> String {
    format!(
        "magnet:?xt=urn:ppspp:{}&x.im=2&x.hf=4&x.sa=2&x.cs=1024&x.ps=64&x.sc=1",
        BASE32_NOPAD.encode(id)
    )
}

/// The lines of a recording.
#[derive(Default)]
struct Lines(String);

impl Lines {
    fn out(&mut self, name: &str, values: &[&[u8]]) {
        self.0.push_str(name);
        for v in values {
            self.0.push(' ');
            if v.is_empty() {
                self.0.push('=');
            } else {
                self.0.push_str(&hex::encode(v));
            }
        }
        self.0.push('\n');
    }

    fn record(&mut self, ns: Namespace, id: u64, m: &impl Message) {
        let ns = u64::try_from(ns.0).unwrap().to_be_bytes();
        self.out("record", &[&ns, &id.to_be_bytes(), &m.encode_to_vec()]);
    }

    async fn table<T: Table>(&mut self, tx: &mut ReadTx<MemoryKv>) {
        for r in T::get_all(tx).await.unwrap() {
            self.record(T::NS, T::id(&r), &r);
        }
    }

    async fn singleton<S: Singleton>(&mut self, tx: &mut ReadTx<MemoryKv>) {
        self.record(S::NS, 0, &S::get(tx).await.unwrap().unwrap());
    }
}

/// Runs `f` in a write transaction of its own: one operation of the
/// recorded scenario.
async fn write<T>(
    rs: &ReplicatedStore<MemoryKv>,
    f: impl AsyncFnOnce(&mut WriteTx<MemoryKv>) -> Result<T, strims_dao::Error>,
) -> T {
    let mut tx = rs.write(NOW).await.unwrap();
    let v = f(&mut tx).await.unwrap();
    tx.commit().await.unwrap();
    v
}

/// Inserts the record `f` makes with a new id, in a transaction of its own.
async fn insert<T: Table>(
    rs: &ReplicatedStore<MemoryKv>,
    f: impl FnOnce(u64) -> T::Record,
) -> T::Record {
    write(rs, async |tx| {
        let id = tx.generate_id().await?;
        T::insert(tx, f(id)).await
    })
    .await
}

/// A whisper in the conversation with `peer_key`, from the holder of the
/// certificate `from`, on the network with key `network_key`.
fn whisper(
    id: u64,
    network_key: &[u8],
    peer_key: &[u8],
    from: &CertificateBody,
    body: &str,
    peer_message_id: u64,
) -> chat::WhisperRecord {
    let state = if peer_key == from.key {
        chat::MessageState::Unread
    } else {
        chat::MessageState::Enqueued
    };
    chat::WhisperRecord {
        id,
        network_key: network_key.to_vec(),
        peer_key: peer_key.to_vec(),
        state: state as i32,
        message: Some(chat::Message {
            server_time: NOW.unix_millis(),
            peer_key: from.key.clone(),
            nick: from.subject.clone(),
            body: body.to_owned(),
            ..Default::default()
        }),
        peer_message_id,
        ..Default::default()
    }
}

/// Inserts a channel of `owner` listed as `title`.
async fn insert_channel(rs: &ReplicatedStore<MemoryKv>, owner: Owner, title: &str) -> VideoChannel {
    let snippet = ListingSnippet {
        title: title.to_owned(),
        description: format!("{title} stream"),
        tags: vec!["video".to_owned()],
        category: "games".to_owned(),
        channel_name: title.to_owned(),
        is_mature: true,
        ..Default::default()
    };
    write(rs, async |tx| {
        let c = new_channel(tx, owner, snippet).await?;
        VideoChannels::insert(tx, c).await
    })
    .await
}

/// Records a profile store: writes it through this crate's API, one
/// operation per transaction, and renders it as `store.hex`'s lines.
///
/// The profile `alice` has a persisted session; a network it runs, whose
/// CA has an invitee, a renewal of the owner's certificate and a released
/// alias; a network it joined, with a membership its root signed, and
/// renamed; bootstrap clients and config;
/// chat settings, with a highlight renamed and one deleted; a whisper
/// thread absorbed by a second one, and whispers in it; a device added and
/// one deleted; the owned network's directory; two chat servers on it, one
/// deleted; a notification, and one deleted; video channels of each owner
/// kind, one deleted, and both video configs; the vnic and autoseed configs
/// with a rule on each network; a swarm cache; and the replication lease.
async fn record_store() -> String {
    let mut out = Lines::default();
    let mut rng = det_rng(1);
    out.out("now", &[&NOW.unix_nanos().to_be_bytes()]);

    let kv = MemoryKv::new();
    let (pk, store) = auth::sign_up(&kv, "alice", "hunter2", "linux", &mut rng, NOW)
        .await
        .unwrap();
    out.out(
        "auth",
        &[b"alice", b"hunter2", &pk.id.to_be_bytes(), &pk.key],
    );

    let session_key: [u8; 32] = seq(200, 32).try_into().unwrap();
    let sk = StorageKey::from_bytes(session_key);
    let session = auth::create_session(&kv, &sk, &pk, &mut rng, NOW)
        .await
        .unwrap();
    out.out(
        "session",
        &[&session_key, &session.eol.to_be_bytes(), &session.token],
    );

    let rs = ReplicatedStore::new(store).await.unwrap();
    let profile = Profile::get(&mut rs.read().await.unwrap())
        .await
        .unwrap()
        .unwrap();
    let profile_key = profile.key.clone().unwrap();

    // Networks: one this profile runs, one it joined.
    let ca_key = generate_key(&mut rng);
    let csr = new_certificate_request(&profile_key, PEER_SIGN, &profile.name).unwrap();
    let membership = sign_certificate_request_with_network(
        &csr,
        &ca_key,
        "owned",
        DEFAULT_CERT_TTL,
        NOW,
        &mut rng,
    )
    .unwrap();
    let server_config = ServerConfig {
        name: "owned".to_owned(),
        key: Some(ca_key.clone()),
        directory: Some(directory::ServerConfig {
            integrations: Some(Integrations {
                swarm: Some(Swarm { enable: true }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let owned = insert::<Networks>(&rs, |id| Network {
        id,
        certificate: Some(membership),
        server_config: Some(server_config),
        ..Default::default()
    })
    .await;
    let owner_cert = CertificateChain::decode(owned.certificate.as_ref().unwrap()).unwrap();
    let owned_key = owner_cert.network_key().to_vec();

    let joined_key = generate_key(&mut rng);
    let joined_root =
        network_certificate(&joined_key, "joined", DEFAULT_CERT_TTL, NOW, &mut rng).unwrap();
    let joined_csr = new_certificate_request(&profile_key, PEER_SIGN, &profile.name).unwrap();
    let joined_cert = sign_certificate_request(
        &joined_csr,
        Duration::HOUR,
        Issuer::Certificate {
            key: &joined_key,
            certificate: &joined_root,
        },
        NOW,
        &mut rng,
    )
    .unwrap();
    let joined_network_key = joined_key.public.clone();
    let joined = insert::<Networks>(&rs, |id| Network {
        id,
        certificate: Some(joined_cert),
        alias: "joined".to_owned(),
        ..Default::default()
    })
    .await;
    write(&rs, async |tx| {
        Networks::transform(tx, joined.id, |n| {
            n.alias = "renamed".to_owned();
            Ok(())
        })
        .await
    })
    .await;

    // The owned network's CA: an invitee, a renewal of the owner's
    // certificate that takes over its subject and key, an alias given up,
    // and the UI's network order.
    let owner_peer = NetworkPeers::index_get(
        &mut rs.read().await.unwrap(),
        &NetworkPeers::BY_PUBLIC_KEY,
        &network_scoped_key(owned.id, &owner_cert.key),
    )
    .await
    .unwrap()
    .unwrap();
    let invitee_key = generate_key(&mut rng);
    let invitee_csr = new_certificate_request(&invitee_key, PEER_SIGN, "invitee").unwrap();
    let invitee_cert = sign_certificate_request_with_network(
        &invitee_csr,
        &ca_key,
        "owned",
        DEFAULT_CERT_TTL,
        NOW,
        &mut rng,
    )
    .unwrap();
    insert::<CertificateLogs>(&rs, |id| new_certificate_log(id, owned.id, &invitee_cert)).await;
    insert::<NetworkPeers>(&rs, |id| {
        new_network_peer(
            id,
            owned.id,
            &invitee_key.public,
            "invitee",
            owner_peer.id,
            NOW,
        )
    })
    .await;
    insert::<AliasReservations>(&rs, |id| {
        new_alias_reservation(id, owned.id, "invitee", &invitee_key.public)
    })
    .await;
    let renewal_csr = new_certificate_request(&profile_key, PEER_SIGN, &profile.name).unwrap();
    let renewed = sign_certificate_request_with_network(
        &renewal_csr,
        &ca_key,
        "owned",
        DEFAULT_CERT_TTL,
        NOW,
        &mut rng,
    )
    .unwrap();
    insert::<CertificateLogs>(&rs, |id| new_certificate_log(id, owned.id, &renewed)).await;
    let spare = insert::<AliasReservations>(&rs, |id| {
        new_alias_reservation(id, owned.id, "spare", &invitee_key.public)
    })
    .await;
    write(&rs, async |tx| {
        AliasReservations::transform(tx, spare.id, |r| {
            r.peer_key.clear();
            r.reserved_until = (NOW + ALIAS_RESERVATION_COOLDOWN).unix();
            Ok(())
        })
        .await
    })
    .await;
    write(&rs, async |tx| {
        NetworkUiConfig::set(
            tx,
            UiConfig {
                network_display_order: vec![joined.id, owned.id],
            },
        )
        .await
    })
    .await;

    // Bootstrap clients.
    for (url, insecure) in [
        ("wss://one.example/ws", false),
        ("wss://two.example/ws", true),
    ] {
        insert::<BootstrapClients>(&rs, |id| BootstrapClient {
            id,
            client_options: Some(ClientOptions::WebsocketOptions(
                BootstrapClientWebSocketOptions {
                    url: url.to_owned(),
                    insecure_skip_verify_tls: insecure,
                },
            )),
            ..Default::default()
        })
        .await;
    }
    write(&rs, async |tx| {
        BootstrapConfig::set(
            tx,
            bootstrap::Config {
                enable_publishing: true,
            },
        )
        .await
    })
    .await;

    // Chat settings.
    write(&rs, async |tx| {
        ChatUiConfig::set(
            tx,
            chat::UiConfig {
                replicate: true,
                show_time: true,
                highlight: true,
                ..Default::default()
            },
        )
        .await
    })
    .await;
    let highlight = |alias: &str, peer_key: &[u8]| {
        let (alias, peer_key) = (alias.to_owned(), peer_key.to_vec());
        move |id| chat::UiConfigHighlight {
            id,
            alias,
            peer_key,
            ..Default::default()
        }
    };
    let h1 = insert::<ChatUiConfigHighlights>(&rs, highlight("first", b"peer-a")).await;
    let h2 = insert::<ChatUiConfigHighlights>(&rs, highlight("second", b"peer-b")).await;
    write(&rs, async |tx| {
        ChatUiConfigHighlights::transform(tx, h1.id, |h| {
            h.alias = "first renamed".to_owned();
            Ok(())
        })
        .await
    })
    .await;
    write(&rs, async |tx| {
        ChatUiConfigHighlights::delete(tx, h2.id).await
    })
    .await;
    insert::<ChatUiConfigTags>(&rs, |id| chat::UiConfigTag {
        id,
        alias: "tagged".to_owned(),
        peer_key: b"peer-c".to_vec(),
        color: "#ff0000".to_owned(),
        ..Default::default()
    })
    .await;
    insert::<ChatUiConfigIgnores>(&rs, |id| chat::UiConfigIgnore {
        id,
        alias: "ignored".to_owned(),
        peer_key: b"peer-d".to_vec(),
        ..Default::default()
    })
    .await;

    // Whispers: a second thread with a peer absorbs the first.
    let peer_key = generate_key(&mut rng);
    let peer_cert = new_self_signed_certificate(
        &peer_key,
        KeyUsage::Peer as i32,
        Duration::HOUR,
        "peer",
        NOW,
        &mut rng,
    )
    .unwrap();
    let peer_cert = certificate_body(&peer_cert).unwrap();
    let thread = |id| chat::WhisperThread {
        id,
        peer_key: peer_cert.key.clone(),
        alias: peer_cert.subject.clone(),
        ..Default::default()
    };
    insert::<WhisperThreads>(&rs, |id| chat::WhisperThread {
        last_message_time: 100,
        has_unread: true,
        ..thread(id)
    })
    .await;
    insert::<WhisperThreads>(&rs, thread).await;
    let own_cert = CertificateBody {
        key: profile_key.public.clone(),
        subject: profile.name.clone(),
        ..Default::default()
    };
    let (network, peer) = (&joined_network_key, &peer_cert.key);
    insert::<WhisperRecords>(&rs, |id| whisper(id, network, peer, &peer_cert, "hi", 5)).await;
    insert::<WhisperRecords>(&rs, |id| whisper(id, network, peer, &own_cert, "hello", 0)).await;
    let again =
        insert::<WhisperRecords>(&rs, |id| whisper(id, network, peer, &peer_cert, "again", 6))
            .await;
    write(&rs, async |tx| {
        WhisperRecords::transform(tx, again.id, |r| {
            r.state = chat::MessageState::Read as i32;
            Ok(())
        })
        .await
    })
    .await;

    // Devices: one added, one added then deleted.
    let device = |name: &str, os: &str| {
        let (name, os) = (name.to_owned(), os.to_owned());
        move |id| Device {
            id,
            device: name,
            os,
            ..Default::default()
        }
    };
    insert::<Devices>(&rs, device("phone", "android")).await;
    let old = insert::<Devices>(&rs, device("old", "windows")).await;
    write(&rs, async |tx| Devices::delete(tx, old.id).await).await;

    // The owned network's directory, whose admin is the owner: a listing
    // of each kind (the media one moderated) and an invitee who may not
    // join.
    let swarm = swarm_uri(&generate_key(&mut rng).public);
    let listings = [
        Content::Chat(listing::Chat {
            key: invitee_key.public.clone(),
            name: "lobby".to_owned(),
        }),
        Content::Embed(listing::Embed {
            service: listing::embed::Service::DirectoryListingEmbedServiceTwitchStream as i32,
            id: "streamer".to_owned(),
            ..Default::default()
        }),
        Content::Media(listing::Media {
            mime_type: "video/mp4".to_owned(),
            swarm_uri: swarm.clone(),
        }),
        Content::Service(listing::Service {
            r#type: "game".to_owned(),
            swarm_uri: swarm,
        }),
    ];
    for content in listings {
        let media = matches!(content, Content::Media(_));
        insert::<DirectoryListingRecords>(&rs, |id| {
            let listing = Listing {
                content: Some(content),
            };
            let mut r = new_listing_record(id, owned.id, listing, None);
            if media {
                r.moderation = Some(ListingModeration {
                    is_mature: Some(true),
                    category: Some("games".to_owned()),
                    ..Default::default()
                });
                r.notes = "checked".to_owned();
            }
            r
        })
        .await;
    }
    insert::<DirectoryUserRecords>(&rs, |id| {
        let moderation = UserModeration {
            disable_join: Some(true),
            ..Default::default()
        };
        new_user_record(id, owned.id, &invitee_key.public, moderation)
    })
    .await;

    // Chat servers on the owned network, each with an icon, two emotes, a
    // modifier, a tag, and profiles of the invitee (muted) and the owner.
    // The second server is deleted, and its icon with it.
    let mut servers = Vec::new();
    for name in ["kept", "deleted"] {
        let key = generate_key(&mut rng);
        let s = insert::<ChatServers>(&rs, |id| chat::Server {
            id,
            network_key: owned_key.clone(),
            key: Some(key),
            room: Some(chat::Room {
                name: name.to_owned(),
                css: format!(".{name} {{}}"),
            }),
            admin_peer_keys: vec![invitee_key.public.clone()],
        })
        .await;
        let icon = chat::ServerIcon {
            id: s.id,
            image: Some(Image {
                r#type: ImageType::Png as i32,
                height: 1,
                width: 1,
                data: name.as_bytes().to_vec(),
            }),
        };
        write(&rs, async |tx| ChatServerIcons::insert(tx, icon).await).await;
        for emote in ["PeepoHappy", "FeelsBadMan"] {
            insert::<ChatEmotes>(&rs, |id| chat::Emote {
                id,
                server_id: s.id,
                name: emote.to_owned(),
                images: vec![chat::EmoteImage {
                    data: emote.as_bytes().to_vec(),
                    file_type: chat::EmoteFileType::FileTypePng as i32,
                    height: 28,
                    width: 28,
                    scale: chat::EmoteScale::EmoteScale2x as i32,
                }],
                effects: vec![chat::EmoteEffect {
                    effect: Some(emote_effect::Effect::SpriteAnimation(
                        emote_effect::SpriteAnimation {
                            frame_count: 4,
                            duration_ms: 400,
                            loop_forever: true,
                            ..Default::default()
                        },
                    )),
                }],
                contributor: Some(chat::EmoteContributor {
                    name: "artist".to_owned(),
                    link: "https://artist.example".to_owned(),
                }),
                labels: vec!["pepe".to_owned()],
                enable: true,
            })
            .await;
        }
        insert::<ChatModifiers>(&rs, |id| chat::Modifier {
            id,
            server_id: s.id,
            name: "wide".to_owned(),
            priority: 1,
            internal: false,
            extra_wrap_count: 2,
            proc_chance: 0.25,
            style_sheet: Some(chat::StyleSheet {
                css: ".wide {}".to_owned(),
                ..Default::default()
            }),
        })
        .await;
        insert::<ChatTags>(&rs, |id| chat::Tag {
            id,
            server_id: s.id,
            name: "nsfw".to_owned(),
            color: "#ff0000".to_owned(),
            sensitive: true,
        })
        .await;
        insert::<ChatProfiles>(&rs, |id| chat::Profile {
            id,
            server_id: s.id,
            peer_key: invitee_key.public.clone(),
            alias: "invitee".to_owned(),
            mutes: vec![chat::profile::Mute {
                created_at: 100,
                duration_secs: 60,
                message: "spam".to_owned(),
                moderator_peer_key: profile_key.public.clone(),
            }],
            mute_deadline: 160,
        })
        .await;
        insert::<ChatProfiles>(&rs, |id| chat::Profile {
            id,
            server_id: s.id,
            peer_key: profile_key.public.clone(),
            alias: profile.name.clone(),
            ..Default::default()
        })
        .await;
        servers.push(s);
    }
    write(&rs, async |tx| ChatServers::delete(tx, servers[1].id).await).await;
    write(&rs, async |tx| {
        ChatServerIcons::delete(tx, servers[1].id).await
    })
    .await;

    // Notifications: a failed renewal on the owned network, and one
    // dismissed.
    insert::<Notifications>(&rs, |id| {
        let subject = Some(network_subject(owned.id));
        let title = "Certificate renewal failed";
        let n = new_notification(id, NOW, Status::Error, title, "rate limited", subject);
        strims_proto::notification::v1::Notification { error_code: 2, ..n }
    })
    .await;
    let dismissed = insert::<Notifications>(&rs, |id| {
        new_notification(id, NOW, Status::Info, "dismissed", "", None)
    })
    .await;
    write(&rs, async |tx| {
        Notifications::delete(tx, dismissed.id).await
    })
    .await;

    // Video channels: the profile's own on the owned network, the
    // invitee's share of it, a remote share on the joined network, and a
    // deleted one. Both configs, the ingress one naming both networks.
    let local_owner = || {
        Owner::Local(Local {
            auth_key: profile_key.public.clone(),
            network_key: owned_key.clone(),
        })
    };
    let local = insert_channel(&rs, local_owner(), "local").await;
    let share_owner = Owner::LocalShare(LocalShare {
        certificate: Some(invitee_cert.clone()),
    });
    let share = insert_channel(&rs, share_owner, "share").await;
    let remote_owner = Owner::RemoteShare(RemoteShare {
        id: 42,
        network_key: joined_network_key.clone(),
        service_key: generate_key(&mut rng).public,
        service_salt: b"salt".to_vec(),
        server_addr: "rtmp.example:1935".to_owned(),
    });
    insert_channel(&rs, remote_owner, "remote").await;
    let deleted = insert_channel(&rs, local_owner(), "deleted").await;
    write(&rs, async |tx| VideoChannels::delete(tx, deleted.id).await).await;
    write(&rs, async |tx| {
        let config = VideoIngressConfig {
            enabled: true,
            server_addr: "0.0.0.0:1935".to_owned(),
            public_server_addr: "rtmp.example:1935".to_owned(),
            service_network_keys: vec![owned_key.clone(), joined_network_key.clone()],
        };
        IngressConfig::set(tx, config).await
    })
    .await;
    write(&rs, async |tx| {
        let config = HlsEgressConfig {
            enabled: true,
            public_server_addr: "https://hls.example".to_owned(),
        };
        HlsConfig::set(tx, config).await
    })
    .await;
    for (c, key) in [(&local, &profile_key), (&share, &invitee_key)] {
        let stream_key = format_stream_key(c.id, &c.token, key).unwrap();
        out.out(
            "video_stream_key",
            &[&c.id.to_be_bytes(), stream_key.as_bytes()],
        );
    }

    // The vnic config, an enabled autoseed config with a rule on each
    // network, and a cache of a swarm under a salt.
    write(&rs, async |tx| {
        let config = vnic::Config {
            max_upload_bytes_per_second: 1 << 20,
            max_peers: 12,
        };
        VnicConfig::set(tx, config).await
    })
    .await;
    write(&rs, async |tx| {
        AutoseedConfig::set(tx, autoseed::Config { enable: true }).await
    })
    .await;
    for (i, network_key) in [&owned_key, &joined_network_key].into_iter().enumerate() {
        let label = format!("rule {i}");
        let rule = autoseed::Rule {
            network_key: network_key.clone(),
            swarm_id: generate_key(&mut rng).public,
            salt: label.clone().into_bytes(),
            label,
            ..Default::default()
        };
        write(&rs, async |tx| {
            let rule = new_rule(tx, rule).await?;
            AutoseedRules::insert(tx, rule).await
        })
        .await;
    }
    let cache_swarm_id = generate_key(&mut rng).public;
    let cache_salt = b"chat:assets";
    let cache = Cache {
        uri: swarm_uri(&cache_swarm_id),
        epoch: Some(Epoch {
            timestamp: 1_700_000_000_000_000_000,
            signature: seq(1, 64),
        }),
        integrity: Some(Integrity {
            merkle_integrity: Some(MerkleIntegrity {
                timestamps: vec![1, 2],
                signatures: vec![seq(64, 64), seq(128, 64)],
            }),
            ..Default::default()
        }),
        data: seq(7, 100),
        ..Default::default()
    };
    write(&rs, async |tx| {
        set_cache(tx, &cache_swarm_id, cache_salt, cache.clone()).await
    })
    .await;
    out.out(
        "swarm_cache",
        &[&cache_swarm_id, cache_salt, &cache.encode_to_vec()],
    );

    let token = lease::new_token(&mut rng);
    lease::try_lock(rs.store(), &lease::key(&["replication"]), &token, NOW)
        .await
        .unwrap();

    for (table, rows) in kv.dump() {
        for (key, value) in rows {
            out.out("kv", &[table.as_bytes(), key.as_bytes(), &value]);
        }
    }

    // The records as this crate reads them.
    let mut tx = rs.read().await.unwrap();
    out.singleton::<Profile>(&mut tx).await;
    out.singleton::<ProfileIds>(&mut tx).await;
    out.singleton::<ChatUiConfig>(&mut tx).await;
    out.table::<Devices>(&mut tx).await;
    out.table::<Networks>(&mut tx).await;
    out.table::<BootstrapClients>(&mut tx).await;
    out.singleton::<NetworkUiConfig>(&mut tx).await;
    out.singleton::<BootstrapConfig>(&mut tx).await;
    out.table::<CertificateLogs>(&mut tx).await;
    out.table::<NetworkPeers>(&mut tx).await;
    out.table::<AliasReservations>(&mut tx).await;
    out.table::<ChatUiConfigHighlights>(&mut tx).await;
    out.table::<ChatUiConfigTags>(&mut tx).await;
    out.table::<ChatUiConfigIgnores>(&mut tx).await;
    out.table::<WhisperThreads>(&mut tx).await;
    out.table::<WhisperRecords>(&mut tx).await;
    out.table::<EventLogs>(&mut tx).await;
    out.table::<Checkpoints>(&mut tx).await;
    out.table::<DirectoryListingRecords>(&mut tx).await;
    out.table::<DirectoryUserRecords>(&mut tx).await;
    out.table::<ChatServers>(&mut tx).await;
    out.table::<ChatServerIcons>(&mut tx).await;
    out.table::<ChatEmotes>(&mut tx).await;
    out.table::<ChatModifiers>(&mut tx).await;
    out.table::<ChatTags>(&mut tx).await;
    out.table::<ChatProfiles>(&mut tx).await;
    out.table::<Notifications>(&mut tx).await;
    out.table::<VideoChannels>(&mut tx).await;
    out.singleton::<IngressConfig>(&mut tx).await;
    out.singleton::<HlsConfig>(&mut tx).await;
    out.singleton::<VnicConfig>(&mut tx).await;
    out.singleton::<AutoseedConfig>(&mut tx).await;
    out.table::<AutoseedRules>(&mut tx).await;
    let metas = SwarmCacheMetas::get_all(&mut tx).await.unwrap();
    for m in &metas {
        out.record(SwarmCacheMetas::NS, m.id, m);
    }
    for m in &metas {
        let cache = SwarmCaches::get(&mut tx, m.id).await.unwrap().unwrap();
        out.record(SwarmCaches::NS, m.id, &cache);
    }
    drop(tx);

    // A replica that has seen the first three of this replica's logs.
    let remote = VersionVector {
        value: [(rs.replica_id(), 3)].into(),
        ..Default::default()
    };
    out.out("delta_remote", &[&remote.encode_to_vec()]);
    for l in compressed_delta(rs.store(), &remote).await.unwrap() {
        out.out("delta", &[&l.encode_to_vec()]);
    }
    out.0
}

/// The recorded store, `store.hex` as [`record_store`] writes it.
fn record() -> String {
    block(record_store())
}

/// `store.hex`'s text. An update run records the file first, so every test
/// reads the new recording.
fn store_text() -> &'static str {
    static TEXT: OnceLock<String> = OnceLock::new();
    TEXT.get_or_init(|| {
        if strims_golden::updating() {
            strims_golden::check_file(PATH, &record());
        }
        std::fs::read_to_string(PATH).unwrap_or_else(|e| panic!("reading {PATH}: {e}"))
    })
}

#[test]
fn recorder_reproduces_the_recorded_store() {
    strims_golden::check_file(PATH, &record());
}

struct Golden {
    now: Time,
    name: String,
    password: String,
    profile: ProfileKey,
    session_key: [u8; 32],
    session: SessionToken,
    dump: Dump,
    records: Vec<(i64, u64, Vec<u8>)>,
    delta_remote: VersionVector,
    delta: Vec<EventLog>,
    /// Recorded stream keys, with the channel each names.
    stream_keys: Vec<(u64, String)>,
    /// Recorded swarm caches, as (swarm id, salt, cache).
    swarm_caches: Vec<(Vec<u8>, Vec<u8>, Cache)>,
}

fn u64_of(b: &[u8]) -> u64 {
    u64::from_be_bytes(b.try_into().unwrap())
}

fn load() -> Golden {
    let mut g = Golden {
        now: Time::EPOCH,
        name: String::new(),
        password: String::new(),
        profile: ProfileKey {
            id: 0,
            key: [0; 32],
        },
        session_key: [0; 32],
        session: SessionToken {
            eol: 0,
            token: vec![],
        },
        dump: Dump::new(),
        records: vec![],
        delta_remote: VersionVector::default(),
        delta: vec![],
        stream_keys: vec![],
        swarm_caches: vec![],
    };
    for line in store_text().lines().filter(|l| !l.trim().is_empty()) {
        let mut parts = line.split(' ');
        let name = parts.next().unwrap();
        let v: Vec<Vec<u8>> = parts
            .map(|p| {
                if p == "=" {
                    vec![]
                } else {
                    hex::decode(p).unwrap()
                }
            })
            .collect();
        match name {
            "now" => g.now = Time::from_unix_nanos(u64_of(&v[0]) as i64),
            "auth" => {
                g.name = String::from_utf8(v[0].clone()).unwrap();
                g.password = String::from_utf8(v[1].clone()).unwrap();
                g.profile = ProfileKey::from_parts(u64_of(&v[2]), &v[3]).unwrap();
            }
            "session" => {
                g.session_key = v[0].as_slice().try_into().unwrap();
                g.session = SessionToken {
                    eol: u64_of(&v[1]),
                    token: v[2].clone(),
                };
            }
            "kv" => {
                g.dump
                    .entry(String::from_utf8(v[0].clone()).unwrap())
                    .or_default()
                    .insert(String::from_utf8(v[1].clone()).unwrap(), v[2].clone());
            }
            "record" => g
                .records
                .push((u64_of(&v[0]) as i64, u64_of(&v[1]), v[2].clone())),
            "delta_remote" => g.delta_remote = VersionVector::decode(v[0].as_slice()).unwrap(),
            "delta" => g.delta.push(EventLog::decode(v[0].as_slice()).unwrap()),
            "video_stream_key" => g
                .stream_keys
                .push((u64_of(&v[0]), String::from_utf8(v[1].clone()).unwrap())),
            "swarm_cache" => g.swarm_caches.push((
                v[0].clone(),
                v[1].clone(),
                Cache::decode(v[2].as_slice()).unwrap(),
            )),
            other => panic!("unknown line {other}"),
        }
    }
    g
}

fn open(g: &Golden) -> (MemoryKv, ProfileStore<MemoryKv>) {
    let kv = MemoryKv::new();
    kv.load(g.dump.clone()).unwrap();
    let store = block(ProfileStore::open(
        kv.clone(),
        g.profile.id,
        g.profile.storage_key(),
        det_rng(1),
    ))
    .unwrap();
    (kv, store)
}

/// `bytes`, a record of namespace `ns`, re-encoded by this crate's types,
/// so encodings compare by content.
fn canonical(ns: i64, bytes: &[u8]) -> Vec<u8> {
    fn re<M: Message + Default>(b: &[u8]) -> Vec<u8> {
        M::decode(b).unwrap().encode_to_vec()
    }
    match Namespace(ns) {
        Profile::NS => re::<<Profile as Singleton>::Record>(bytes),
        ProfileIds::NS => re::<<ProfileIds as Singleton>::Record>(bytes),
        ChatUiConfig::NS => re::<<ChatUiConfig as Singleton>::Record>(bytes),
        Devices::NS => re::<<Devices as Table>::Record>(bytes),
        Networks::NS => re::<<Networks as Table>::Record>(bytes),
        BootstrapClients::NS => re::<<BootstrapClients as Table>::Record>(bytes),
        NetworkUiConfig::NS => re::<<NetworkUiConfig as Singleton>::Record>(bytes),
        BootstrapConfig::NS => re::<<BootstrapConfig as Singleton>::Record>(bytes),
        CertificateLogs::NS => re::<<CertificateLogs as Table>::Record>(bytes),
        NetworkPeers::NS => re::<<NetworkPeers as Table>::Record>(bytes),
        AliasReservations::NS => re::<<AliasReservations as Table>::Record>(bytes),
        ChatUiConfigHighlights::NS => re::<<ChatUiConfigHighlights as Table>::Record>(bytes),
        ChatUiConfigTags::NS => re::<<ChatUiConfigTags as Table>::Record>(bytes),
        ChatUiConfigIgnores::NS => re::<<ChatUiConfigIgnores as Table>::Record>(bytes),
        WhisperThreads::NS => re::<<WhisperThreads as Table>::Record>(bytes),
        WhisperRecords::NS => re::<<WhisperRecords as Table>::Record>(bytes),
        EventLogs::NS => re::<<EventLogs as Table>::Record>(bytes),
        Checkpoints::NS => re::<<Checkpoints as Table>::Record>(bytes),
        DirectoryListingRecords::NS => re::<<DirectoryListingRecords as Table>::Record>(bytes),
        DirectoryUserRecords::NS => re::<<DirectoryUserRecords as Table>::Record>(bytes),
        ChatServers::NS => re::<<ChatServers as Table>::Record>(bytes),
        ChatServerIcons::NS => re::<<ChatServerIcons as Table>::Record>(bytes),
        ChatEmotes::NS => re::<<ChatEmotes as Table>::Record>(bytes),
        ChatModifiers::NS => re::<<ChatModifiers as Table>::Record>(bytes),
        ChatTags::NS => re::<<ChatTags as Table>::Record>(bytes),
        ChatProfiles::NS => re::<<ChatProfiles as Table>::Record>(bytes),
        Notifications::NS => re::<<Notifications as Table>::Record>(bytes),
        VideoChannels::NS => re::<<VideoChannels as Table>::Record>(bytes),
        IngressConfig::NS => re::<<IngressConfig as Singleton>::Record>(bytes),
        HlsConfig::NS => re::<<HlsConfig as Singleton>::Record>(bytes),
        VnicConfig::NS => re::<<VnicConfig as Singleton>::Record>(bytes),
        AutoseedConfig::NS => re::<<AutoseedConfig as Singleton>::Record>(bytes),
        AutoseedRules::NS => re::<<AutoseedRules as Table>::Record>(bytes),
        SwarmCacheMetas::NS => re::<<SwarmCacheMetas as Table>::Record>(bytes),
        SwarmCaches::NS => re::<<SwarmCaches as Table>::Record>(bytes),
        other => panic!("no record type for {other}"),
    }
}

/// Record `id` of namespace `ns`, as this crate reads it.
fn read(tx: &mut ReadTx<MemoryKv>, ns: i64, id: u64) -> Option<Vec<u8>> {
    fn t<T: Table>(tx: &mut ReadTx<MemoryKv>, id: u64) -> Option<Vec<u8>> {
        block(T::get(tx, id)).unwrap().map(|r| r.encode_to_vec())
    }
    fn s<S: Singleton>(tx: &mut ReadTx<MemoryKv>) -> Option<Vec<u8>> {
        block(S::get(tx)).unwrap().map(|r| r.encode_to_vec())
    }
    match Namespace(ns) {
        Profile::NS => s::<Profile>(tx),
        ProfileIds::NS => s::<ProfileIds>(tx),
        ChatUiConfig::NS => s::<ChatUiConfig>(tx),
        Devices::NS => t::<Devices>(tx, id),
        Networks::NS => t::<Networks>(tx, id),
        BootstrapClients::NS => t::<BootstrapClients>(tx, id),
        NetworkUiConfig::NS => s::<NetworkUiConfig>(tx),
        BootstrapConfig::NS => s::<BootstrapConfig>(tx),
        CertificateLogs::NS => t::<CertificateLogs>(tx, id),
        NetworkPeers::NS => t::<NetworkPeers>(tx, id),
        AliasReservations::NS => t::<AliasReservations>(tx, id),
        ChatUiConfigHighlights::NS => t::<ChatUiConfigHighlights>(tx, id),
        ChatUiConfigTags::NS => t::<ChatUiConfigTags>(tx, id),
        ChatUiConfigIgnores::NS => t::<ChatUiConfigIgnores>(tx, id),
        WhisperThreads::NS => t::<WhisperThreads>(tx, id),
        WhisperRecords::NS => t::<WhisperRecords>(tx, id),
        EventLogs::NS => t::<EventLogs>(tx, id),
        Checkpoints::NS => t::<Checkpoints>(tx, id),
        DirectoryListingRecords::NS => t::<DirectoryListingRecords>(tx, id),
        DirectoryUserRecords::NS => t::<DirectoryUserRecords>(tx, id),
        ChatServers::NS => t::<ChatServers>(tx, id),
        ChatServerIcons::NS => t::<ChatServerIcons>(tx, id),
        ChatEmotes::NS => t::<ChatEmotes>(tx, id),
        ChatModifiers::NS => t::<ChatModifiers>(tx, id),
        ChatTags::NS => t::<ChatTags>(tx, id),
        ChatProfiles::NS => t::<ChatProfiles>(tx, id),
        Notifications::NS => t::<Notifications>(tx, id),
        VideoChannels::NS => t::<VideoChannels>(tx, id),
        IngressConfig::NS => s::<IngressConfig>(tx),
        HlsConfig::NS => s::<HlsConfig>(tx),
        VnicConfig::NS => s::<VnicConfig>(tx),
        AutoseedConfig::NS => s::<AutoseedConfig>(tx),
        AutoseedRules::NS => t::<AutoseedRules>(tx, id),
        SwarmCacheMetas::NS => t::<SwarmCacheMetas>(tx, id),
        SwarmCaches::NS => t::<SwarmCaches>(tx, id),
        other => panic!("no record type for {other}"),
    }
}

#[test]
fn sign_in_and_session_records_open() {
    let g = load();
    let kv = MemoryKv::new();
    kv.load(g.dump.clone()).unwrap();
    assert_eq!(
        block(auth::sign_in(&kv, &g.name, &g.password)).unwrap(),
        g.profile
    );
    assert!(block(auth::sign_in(&kv, &g.name, "wrong")).is_err());
    let sk = StorageKey::from_bytes(g.session_key);
    assert_eq!(
        block(auth::load_session(&kv, &sk, &g.session, g.now)).unwrap(),
        g.profile
    );
}

#[test]
fn records_read_back() {
    let g = load();
    let (_, store) = open(&g);
    let mut tx = block(store.read()).unwrap();
    for (ns, id, bytes) in &g.records {
        let got = read(&mut tx, *ns, *id).unwrap_or_else(|| panic!("missing {ns}/{id}"));
        assert_eq!(got, canonical(*ns, bytes), "{ns}/{id}");
    }
    let profile = block(Profile::get(&mut tx)).unwrap().unwrap();
    assert_eq!(profile.name, g.name);
    assert_ne!(profile.device_id, 0);
    let notifications = block(Notifications::get_all(&mut tx)).unwrap();
    assert_eq!(notifications.len(), 1);
    assert!(notifications[0].subject.is_some());
}

/// The keys of `T`'s rows and index entries. A record whose unique key
/// another record took over has no entry under it, so a unique entry is
/// expected only for the record the index finds.
fn keys_of<T: Table>(tx: &mut ReadTx<MemoryKv>, salt: &[u8], out: &mut BTreeSet<String>) {
    for r in block(T::get_all(tx)).unwrap() {
        let id = T::id(&r);
        out.insert(T::NS.record_key(id));
        for ix in T::INDEXES {
            let key = (ix.key)(&r);
            if !(ix.condition)(&r)
                || ix.unique && !block(T::index_ids(tx, ix, &key)).unwrap().contains(&id)
            {
                continue;
            }
            out.insert(index_entry_key(ix.ns, salt, &key, id));
        }
    }
}

#[test]
fn every_key_is_the_one_this_crate_derives() {
    let g = load();
    let (kv, store) = open(&g);
    let salt = g.profile.key;
    let mut tx = block(store.read()).unwrap();

    let mut derived = BTreeSet::new();
    keys_of::<Devices>(&mut tx, &salt, &mut derived);
    keys_of::<Networks>(&mut tx, &salt, &mut derived);
    keys_of::<BootstrapClients>(&mut tx, &salt, &mut derived);
    keys_of::<CertificateLogs>(&mut tx, &salt, &mut derived);
    keys_of::<NetworkPeers>(&mut tx, &salt, &mut derived);
    keys_of::<AliasReservations>(&mut tx, &salt, &mut derived);
    keys_of::<ChatUiConfigHighlights>(&mut tx, &salt, &mut derived);
    keys_of::<ChatUiConfigTags>(&mut tx, &salt, &mut derived);
    keys_of::<ChatUiConfigIgnores>(&mut tx, &salt, &mut derived);
    keys_of::<WhisperThreads>(&mut tx, &salt, &mut derived);
    keys_of::<WhisperRecords>(&mut tx, &salt, &mut derived);
    keys_of::<EventLogs>(&mut tx, &salt, &mut derived);
    keys_of::<Checkpoints>(&mut tx, &salt, &mut derived);
    keys_of::<DirectoryListingRecords>(&mut tx, &salt, &mut derived);
    keys_of::<DirectoryUserRecords>(&mut tx, &salt, &mut derived);
    keys_of::<ChatServers>(&mut tx, &salt, &mut derived);
    keys_of::<ChatServerIcons>(&mut tx, &salt, &mut derived);
    keys_of::<ChatEmotes>(&mut tx, &salt, &mut derived);
    keys_of::<ChatModifiers>(&mut tx, &salt, &mut derived);
    keys_of::<ChatTags>(&mut tx, &salt, &mut derived);
    keys_of::<ChatProfiles>(&mut tx, &salt, &mut derived);
    keys_of::<Notifications>(&mut tx, &salt, &mut derived);
    keys_of::<VideoChannels>(&mut tx, &salt, &mut derived);
    keys_of::<AutoseedRules>(&mut tx, &salt, &mut derived);
    keys_of::<SwarmCacheMetas>(&mut tx, &salt, &mut derived);
    // A cache row is under its meta's id.
    for m in block(SwarmCacheMetas::get_all(&mut tx)).unwrap() {
        derived.insert(SwarmCaches::NS.record_key(m.id));
    }
    for ns in [
        StoreVersions::NS,
        Profile::NS,
        ProfileIds::NS,
        ChatUiConfig::NS,
        NetworkUiConfig::NS,
        BootstrapConfig::NS,
        IngressConfig::NS,
        HlsConfig::NS,
        VnicConfig::NS,
        AutoseedConfig::NS,
    ] {
        derived.insert(ns.key());
    }
    let lease_key = lease::key(&["replication"]);
    derived.insert(lease_key.clone());

    // Every namespace this crate defines, and the lease namespace.
    let mut known: BTreeSet<i64> = [1001, 1002, 1003, 1004, 2001, 2002, 2003].into();
    known.extend(4001..=4018);
    known.insert(5001);
    known.extend(6001..=6025);
    known.extend(7001..=7004);
    known.insert(8001);
    known.extend(9001..=9006);
    known.extend(10001..=10002);
    known.extend(11001..=11003);

    let table = &kv.dump()[&strims_dao::key::profile_table(g.profile.id)];
    let stored: BTreeSet<String> = table
        .keys()
        .filter(|k| {
            let ns = k.split(':').next().and_then(parse_base36).unwrap();
            known.contains(&(ns as i64))
        })
        .cloned()
        .collect();
    let missing: Vec<_> = stored.difference(&derived).collect();
    let extra: Vec<_> = derived.difference(&stored).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "missing {missing:?}, extra {extra:?}"
    );

    // Index entries name their key and record.
    let sk = g.profile.storage_key();
    for (k, v) in table {
        let ns = k.split(':').next().and_then(parse_base36).unwrap() as i64;
        let index_ns = [
            4002, 4004, 4005, 4006, 4010, 4011, 4012, 4013, 4014, 4017, 4018,
        ];
        if index_ns.contains(&ns)
            || [6003, 6005, 6007, 6010, 6011].contains(&ns)
            || [6013, 6014, 6016, 6017, 6019, 6021, 6023, 6025].contains(&ns)
            || [7002, 9002, 9003, 9005, 9006, 11002].contains(&ns)
        {
            let entry = SecondaryIndexKey::decode(sk.open(v).unwrap().as_slice()).unwrap();
            assert_eq!(
                index_entry_key(Namespace(ns), &salt, &entry.key, entry.id),
                *k
            );
        }
    }
    let lease = Mutex::decode(sk.open(&table[&lease_key]).unwrap().as_slice()).unwrap();
    assert_eq!(lease.token.len(), 16);
    assert!(lease.eol > g.now.unix_nanos());
}

/// An event as (namespace, id, delete, encoded version, canonical record).
type EventKey = (i64, u64, bool, Vec<u8>, Vec<u8>);

/// Logs by id: the encoded checkpoint and the sorted events.
fn normalize(logs: &[EventLog]) -> BTreeMap<u64, (Vec<u8>, Vec<EventKey>)> {
    logs.iter()
        .map(|l| {
            let mut events: Vec<_> = l
                .events
                .iter()
                .map(|e: &Event| {
                    let record = if e.delete {
                        vec![]
                    } else {
                        canonical(e.namespace, &e.record)
                    };
                    (
                        e.namespace,
                        e.id,
                        e.delete,
                        e.version.clone().unwrap_or_default().encode_to_vec(),
                        record,
                    )
                })
                .collect();
            events.sort();
            (
                l.id,
                (
                    l.checkpoint.clone().unwrap_or_default().encode_to_vec(),
                    events,
                ),
            )
        })
        .collect()
}

#[test]
fn compressed_delta_matches() {
    let g = load();
    let (_, store) = open(&g);
    let delta = block(compressed_delta(&store, &g.delta_remote)).unwrap();
    assert!(!delta.is_empty());
    assert_eq!(normalize(&delta), normalize(&g.delta));
}

/// The owned network's CA records: the owner's chain and its renewal, an
/// invitee under the owner, and an alias given up.
#[test]
fn ca_records_read_back_by_index() {
    let g = load();
    let (_, store) = open(&g);
    let mut tx = block(store.read()).unwrap();
    let owned = block(Networks::get_all(&mut tx))
        .unwrap()
        .into_iter()
        .find(|n| n.server_config.is_some())
        .unwrap();
    let owner_cert = owned.certificate.clone().unwrap();
    let owner_body = certificate_body(&owner_cert).unwrap();
    let scoped = |b: &[u8]| network_scoped_key(owned.id, b);
    let log_body = |l: &CertificateLog| certificate_body(l.certificate.as_ref().unwrap()).unwrap();

    let logs = block(CertificateLogs::index_get_all(
        &mut tx,
        &CertificateLogs::BY_NETWORK,
        &ref_key(owned.id),
    ))
    .unwrap();
    // the owner's member and root certificates, the invitee's, the renewal
    assert_eq!(logs.len(), 4);
    let by_subject = |tx: &mut ReadTx<MemoryKv>, subject: &str| {
        block(CertificateLogs::index_get(
            tx,
            &CertificateLogs::BY_SUBJECT,
            &scoped(subject.as_bytes()),
        ))
        .unwrap()
        .unwrap()
    };
    let latest = log_body(&by_subject(&mut tx, &g.name));
    assert_ne!(latest.serial_number, owner_body.serial_number);
    assert_eq!(latest.key, owner_body.key);
    let by_key = block(CertificateLogs::index_get(
        &mut tx,
        &CertificateLogs::BY_KEY,
        &scoped(&owner_body.key),
    ))
    .unwrap()
    .unwrap();
    assert_eq!(log_body(&by_key).serial_number, latest.serial_number);
    let invitee_log = by_subject(&mut tx, "invitee");

    // Logs hold no parents: each names its issuer by serial number, and
    // only the root is self-signed. The owner's first certificate finds
    // its root's log by the serial number it names.
    assert!(
        logs.iter()
            .all(|l| l.certificate.as_ref().unwrap().parent.is_none())
    );
    let self_signed: Vec<_> = logs
        .iter()
        .map(log_body)
        .filter(|b| b.issuer_serial_number.is_empty())
        .collect();
    assert_eq!(self_signed.len(), 1);
    assert_eq!(self_signed[0].key, network_key(&owned));
    let by_serial = |tx: &mut ReadTx<MemoryKv>, serial: &[u8]| {
        block(CertificateLogs::index_get(
            tx,
            &CertificateLogs::BY_SERIAL_NUMBER,
            &scoped(serial),
        ))
        .unwrap()
        .unwrap()
        .certificate
        .unwrap()
    };
    let member = by_serial(&mut tx, &owner_body.serial_number);
    assert_eq!(
        member,
        Certificate {
            parent: None,
            ..owner_cert.clone()
        }
    );
    let root = by_serial(&mut tx, &owner_body.issuer_serial_number);
    assert_eq!(Some(&root), certificate_parent(&owner_cert));

    let owner = block(NetworkPeers::index_get(
        &mut tx,
        &NetworkPeers::BY_PUBLIC_KEY,
        &scoped(&owner_body.key),
    ))
    .unwrap()
    .unwrap();
    assert_eq!(owner.inviter_peer_id, 0);
    let invited = block(NetworkPeers::index_get_all(
        &mut tx,
        &NetworkPeers::BY_INVITER,
        &ref_key(owner.id),
    ))
    .unwrap();
    assert_eq!(invited.len(), 1);
    assert_eq!(invited[0].public_key, log_body(&invitee_log).key);

    let held = block(AliasReservations::index_get(
        &mut tx,
        &AliasReservations::BY_ALIAS,
        &scoped(b"invitee"),
    ))
    .unwrap()
    .unwrap();
    assert_eq!(held.reserved_until, RESERVED_FOREVER);
    let spare = block(AliasReservations::index_get(
        &mut tx,
        &AliasReservations::BY_ALIAS,
        &scoped(b"spare"),
    ))
    .unwrap()
    .unwrap();
    assert!(spare.peer_key.is_empty());
    assert!(spare.reserved_until > g.now.unix() && spare.reserved_until < RESERVED_FOREVER);

    let ui = block(NetworkUiConfig::get(&mut tx)).unwrap().unwrap();
    assert_eq!(ui.network_display_order.len(), 2);
    assert_eq!(ui.network_display_order[1], owned.id);
    let bootstrap = block(BootstrapConfig::get(&mut tx)).unwrap().unwrap();
    assert!(bootstrap.enable_publishing);
}

/// Both networks hold the profile's membership, signed by the network's
/// root and trusted at the recording time; the owned network's root is its
/// server config's key.
#[test]
fn network_certificates_verify_at_the_recording_time() {
    let g = load();
    let (_, store) = open(&g);
    let mut tx = block(store.read()).unwrap();
    let profile_key = block(Profile::get(&mut tx)).unwrap().unwrap().key.unwrap();
    let networks = block(Networks::get_all(&mut tx)).unwrap();
    assert_eq!(networks.len(), 2);
    for n in &networks {
        let chain = verify_certificate(n.certificate.as_ref().unwrap(), g.now).unwrap();
        assert_eq!(chain.key, profile_key.public);
        assert_eq!(chain.subject, g.name);
        assert_eq!(chain.bodies().len(), 2);
        assert!(chain.membership_trusted(g.now));
        assert_eq!(chain.network_key(), network_key(n));
        match &n.server_config {
            Some(config) => {
                assert_eq!(chain.root().subject, config.name);
                assert_eq!(chain.network_key(), config.key.as_ref().unwrap().public);
            }
            None => assert_eq!(chain.root().subject, "joined"),
        }
    }
}

/// The owned network's directory: the owner as its admin, an invitee who
/// may not join, and a listing of each kind, each found by its key.
#[test]
fn directory_records_read_back_by_index() {
    use strims_proto::network::v1::directory::listing::Content;

    let g = load();
    let (_, store) = open(&g);
    let mut tx = block(store.read()).unwrap();
    let owned = block(Networks::get_all(&mut tx))
        .unwrap()
        .into_iter()
        .find(|n| n.server_config.is_some())
        .unwrap();
    let owner_key = certificate_body(owned.certificate.as_ref().unwrap())
        .unwrap()
        .key;

    let users = block(DirectoryUserRecords::index_get_all(
        &mut tx,
        &DirectoryUserRecords::BY_NETWORK,
        &ref_key(owned.id),
    ))
    .unwrap();
    assert_eq!(users.len(), 2);
    let owner = block(DirectoryUserRecords::index_get(
        &mut tx,
        &DirectoryUserRecords::BY_PEER_KEY,
        &network_scoped_key(owned.id, &owner_key),
    ))
    .unwrap()
    .unwrap();
    let m = owner.moderation.unwrap();
    assert_eq!((m.is_admin, m.is_moderator), (Some(true), Some(true)));
    assert_eq!((m.disable_join, m.disable_publish), (None, None));
    let invitee = users.into_iter().find(|u| u.id != owner.id).unwrap();
    assert_eq!(invitee.moderation.unwrap().disable_join, Some(true));

    let listings = block(DirectoryListingRecords::index_get_all(
        &mut tx,
        &DirectoryListingRecords::BY_NETWORK,
        &ref_key(owned.id),
    ))
    .unwrap();
    assert_eq!(listings.len(), 4);
    for r in &listings {
        let key = listing_key(owned.id, r.listing.as_ref());
        let found = block(DirectoryListingRecords::index_get(
            &mut tx,
            &DirectoryListingRecords::BY_LISTING,
            &key,
        ))
        .unwrap()
        .unwrap();
        assert_eq!(found.id, r.id);
        if let Some(Content::Media(media)) = r.listing.as_ref().and_then(|l| l.content.as_ref()) {
            let id = strims_ppspp::Uri::parse(&media.swarm_uri).unwrap().id;
            assert_eq!(key.get(9..).unwrap(), id.as_bytes());
            let m = found.moderation.unwrap();
            assert_eq!(
                (m.is_mature, m.category.as_deref()),
                (Some(true), Some("games"))
            );
            assert_eq!(found.notes, "checked");
        }
    }
}

/// The owned network's chat server, with its icon, emotes, modifier, tag
/// and profiles, each found by index; the deleted server left nothing.
#[test]
fn chat_records_read_back_by_index() {
    let g = load();
    let (_, store) = open(&g);
    let mut tx = block(store.read()).unwrap();
    let owned = block(Networks::get_all(&mut tx))
        .unwrap()
        .into_iter()
        .find(|n| n.server_config.is_some())
        .unwrap();
    let owner_key = certificate_body(owned.certificate.as_ref().unwrap())
        .unwrap()
        .key;

    let servers = block(ChatServers::get_all(&mut tx)).unwrap();
    assert_eq!(servers.len(), 1);
    let server = &servers[0];
    assert_eq!(server.network_key, network_key(&owned));
    assert_eq!(server.room.as_ref().unwrap().name, "kept");
    let icon = block(ChatServerIcons::get(&mut tx, server.id))
        .unwrap()
        .unwrap();
    assert_eq!(icon.image.unwrap().data, b"kept");
    assert_eq!(block(ChatServerIcons::get_all(&mut tx)).unwrap().len(), 1);

    let k = ref_key(server.id);
    let emotes = block(ChatEmotes::index_get_all(
        &mut tx,
        &ChatEmotes::BY_SERVER,
        &k,
    ))
    .unwrap();
    let mut names: Vec<_> = emotes.iter().map(|e| e.name.as_str()).collect();
    names.sort();
    assert_eq!(names, ["FeelsBadMan", "PeepoHappy"]);
    assert_eq!(block(ChatEmotes::get_all(&mut tx)).unwrap().len(), 2);
    let modifiers = block(ChatModifiers::index_get_all(
        &mut tx,
        &ChatModifiers::BY_SERVER,
        &k,
    ))
    .unwrap();
    assert_eq!(modifiers.len(), 1);
    assert_eq!(modifiers[0].proc_chance, 0.25);
    assert_eq!(block(ChatModifiers::get_all(&mut tx)).unwrap().len(), 1);
    let tags = block(ChatTags::index_get_all(&mut tx, &ChatTags::BY_SERVER, &k)).unwrap();
    assert_eq!(tags.len(), 1);
    assert!(tags[0].sensitive);
    assert_eq!(block(ChatTags::get_all(&mut tx)).unwrap().len(), 1);

    let profiles = block(ChatProfiles::index_get_all(
        &mut tx,
        &ChatProfiles::BY_SERVER,
        &k,
    ))
    .unwrap();
    assert_eq!(profiles.len(), 2);
    assert_eq!(block(ChatProfiles::get_all(&mut tx)).unwrap().len(), 2);
    for p in &profiles {
        let found = block(ChatProfiles::index_get(
            &mut tx,
            &ChatProfiles::BY_PEER_KEY,
            &profile_peer_key(server.id, &p.peer_key),
        ))
        .unwrap()
        .unwrap();
        assert_eq!(found.id, p.id);
    }
    let owner = block(ChatProfiles::index_get(
        &mut tx,
        &ChatProfiles::BY_PEER_KEY,
        &profile_peer_key(server.id, &owner_key),
    ))
    .unwrap()
    .unwrap();
    assert_eq!(owner.alias, g.name);
    let muted = profiles.iter().find(|p| p.id != owner.id).unwrap();
    assert_eq!(muted.mute_deadline, 160);
    assert_eq!(muted.mutes[0].moderator_peer_key, owner_key);
}

/// The video channels, one of each owner kind, each found by its owner
/// key; the deleted one left nothing; both configs; and the recorded stream
/// keys name their channels.
#[test]
fn video_records_read_back() {
    use strims_proto::video::v1::video_channel::Owner;

    let g = load();
    let (_, store) = open(&g);
    let mut tx = block(store.read()).unwrap();
    let networks = block(Networks::get_all(&mut tx)).unwrap();
    let owned = networks.iter().find(|n| n.server_config.is_some()).unwrap();
    let joined = networks.iter().find(|n| n.server_config.is_none()).unwrap();
    let (owned_key, joined_key) = (network_key(owned), network_key(joined));

    let channels = block(VideoChannels::get_all(&mut tx)).unwrap();
    assert_eq!(channels.len(), 3);
    for c in &channels {
        let found = block(VideoChannels::index_get(
            &mut tx,
            &VideoChannels::BY_OWNER,
            &owner_key(c),
        ))
        .unwrap()
        .unwrap();
        assert_eq!(found.id, c.id);
        assert_eq!(c.token.len(), 32);
        assert_eq!(c.key.as_ref().unwrap().public.len(), 32);
        let title = &c.directory_listing_snippet.as_ref().unwrap().title;
        let expected = match c.owner.as_ref().unwrap() {
            Owner::Local(_) => ("local", &owned_key),
            Owner::LocalShare(s) => {
                // the invitee's membership, as the CA issued it
                let chain = verify_certificate(s.certificate.as_ref().unwrap(), g.now).unwrap();
                assert!(chain.membership_trusted(g.now));
                ("share", &owned_key)
            }
            Owner::RemoteShare(r) => {
                assert_eq!(r.server_addr, "rtmp.example:1935");
                ("remote", &joined_key)
            }
        };
        assert_eq!(
            (title.as_str(), channel_network_key(c)),
            (expected.0, Some(expected.1.clone()))
        );
    }

    assert_eq!(g.stream_keys.len(), 2);
    for (id, key) in &g.stream_keys {
        let found = block(channel_by_stream_key(&mut tx, key)).unwrap().unwrap();
        assert_eq!(found.id, *id);
    }

    let ingress = block(IngressConfig::get(&mut tx)).unwrap().unwrap();
    assert!(ingress.enabled);
    assert_eq!(ingress.server_addr, "0.0.0.0:1935");
    assert_eq!(ingress.service_network_keys, [owned_key, joined_key]);
    let hls = block(HlsConfig::get(&mut tx)).unwrap().unwrap();
    assert!(hls.enabled);
    assert_eq!(hls.public_server_addr, "https://hls.example");
}

/// The vnic config, the enabled autoseed config with a rule on each
/// network, and a swarm cache found by its swarm id and salt, whose
/// checksum is the one this crate derives.
#[test]
fn vnic_autoseed_and_swarm_cache_records_read_back() {
    let g = load();
    let (_, store) = open(&g);
    let mut tx = block(store.read()).unwrap();
    let vnic = block(VnicConfig::get(&mut tx)).unwrap().unwrap();
    assert_eq!(
        (vnic.max_upload_bytes_per_second, vnic.max_peers),
        (1 << 20, 12)
    );
    assert!(block(AutoseedConfig::get(&mut tx)).unwrap().unwrap().enable);
    let networks = block(Networks::get_all(&mut tx)).unwrap();
    let owned = networks.iter().find(|n| n.server_config.is_some()).unwrap();
    let joined = networks.iter().find(|n| n.server_config.is_none()).unwrap();
    let mut rules = block(AutoseedRules::get_all(&mut tx)).unwrap();
    rules.sort_by(|a, b| a.label.cmp(&b.label));
    assert_eq!(rules.len(), 2);
    for (r, (label, network)) in rules.iter().zip([("rule 0", owned), ("rule 1", joined)]) {
        assert_eq!(r.label, label);
        assert_eq!(r.salt, label.as_bytes());
        assert_eq!(r.network_key, network_key(network));
        assert_eq!(r.swarm_id.len(), 32);
    }

    assert_eq!(g.swarm_caches.len(), 1);
    let (swarm_id, salt, cache) = &g.swarm_caches[0];
    assert_eq!(salt, b"chat:assets");
    let got = block(get_cache(&mut tx, swarm_id, salt)).unwrap().unwrap();
    assert_eq!(
        got,
        Cache {
            id: got.id,
            ..cache.clone()
        }
    );
    let meta = block(SwarmCacheMetas::index_get(
        &mut tx,
        &SwarmCacheMetas::BY_KEY,
        &cache_key(swarm_id, salt),
    ))
    .unwrap()
    .unwrap();
    assert_eq!(meta.id, got.id);
    assert_eq!(meta.checksum, cache_checksum(cache));
    drop(tx);

    let mut tx = block(store.write(g.now)).unwrap();
    assert!(!block(set_cache(&mut tx, swarm_id, salt, cache.clone())).unwrap());
    // deleting a network deletes its rule
    block(Networks::delete(&mut tx, owned.id)).unwrap();
    let left = block(AutoseedRules::get_all(&mut tx)).unwrap();
    assert_eq!(left, [rules[1].clone()]);
}
