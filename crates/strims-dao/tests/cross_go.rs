// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A store this crate writes, checked by the Go dao (`go run -tags golden
//! ./crates/strims-dao/golden -verify <file>`): sign-in and session
//! records, records and index lookups through the Go API, deleted records
//! (a deleted network's notification among them) absent, a pairing token import, and the store's replication logs
//! applied to a Go replica. Skipped when `go` isn't on PATH.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    // The test runs a Go subprocess and reads the clock for session expiry.
    clippy::disallowed_methods
)]

use std::fmt::Write as _;
use std::future::Future;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;

use futures_util::{FutureExt, StreamExt};
use prost::Message;
use strims_base::rng::det_rng;
use strims_base::time::Time;
use strims_dao::auth::{self, ProfileKey};
use strims_dao::replication::allocate_ids;
use strims_dao::schema::chat::{
    ChatEmotes, ChatModifiers, ChatProfiles, ChatServerIcons, ChatServers, ChatTags, ChatUiConfig,
    ChatUiConfigHighlights, ChatUiConfigIgnores, ChatUiConfigTags, WhisperRecords, WhisperThreads,
    profile_peer_key, state_key,
};
use strims_dao::schema::directory::{
    DirectoryListingRecords, DirectoryUserRecords, listing_key, new_listing_record, new_user_record,
};
use strims_dao::schema::network::{
    AliasReservations, BootstrapClients, CertificateLogs, NetworkPeers, NetworkUiConfig, Networks,
    network_scoped_key, new_alias_reservation, new_network_peer,
};
use strims_dao::schema::notification::{
    Notifications, dispatch, network_subject, new_notification,
};
use strims_dao::schema::profile::{Devices, Profile, ProfileIds};
use strims_dao::schema::system::{Checkpoints, EventLogs};
use strims_dao::seal::StorageKey;
use strims_dao::{
    ReadTx, Recv, ReplicatedStore, Singleton, StoreEvent, Table, WriteTx, lease, ref_key,
};
use strims_kv::MemoryKv;
use strims_proto::auth::v1::PairingToken;
use strims_proto::chat::v1::{
    Emote, EmoteFileType, EmoteImage, MessageState, Modifier, Profile as ChatProfile, Room, Server,
    ServerIcon, Tag, UiConfig, UiConfigHighlight, UiConfigIgnore, UiConfigTag, WhisperRecord,
    WhisperThread, profile::Mute,
};
use strims_proto::network::v1::bootstrap::{
    BootstrapClient, BootstrapClientWebSocketOptions, bootstrap_client,
};
use strims_proto::network::v1::directory::{
    Listing, ListingModeration, UserModeration, listing as directory_listing,
};
use strims_proto::network::v1::{Network, ServerConfig, UiConfig as NetworkUiConfigRecord};
use strims_proto::notification::v1::notification::Status;
use strims_proto::profile::v1::Device;
use strims_proto::replication::v1::EventLog;
use strims_proto::types::{Image, ImageType, Key, KeyUsage};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn go_available() -> bool {
    match Command::new("go").arg("version").output() {
        Ok(o) if o.status.success() => true,
        _ => {
            eprintln!("skipping: `go` is not on PATH");
            false
        }
    }
}

fn block<T>(f: impl Future<Output = T>) -> T {
    f.now_or_never()
        .expect("memory store operations are ready immediately")
}

struct Lines(String);

impl Lines {
    fn put(&mut self, name: &str, values: &[&[u8]]) {
        self.0.push_str(name);
        for v in values {
            if v.is_empty() {
                self.0.push_str(" =");
            } else {
                write!(self.0, " {}", hex::encode(v)).unwrap();
            }
        }
        self.0.push('\n');
    }
}

fn record_lines<T: Table>(tx: &mut ReadTx<MemoryKv>, out: &mut Lines) {
    for r in block(T::get_all(tx)).unwrap() {
        out.put(
            "record",
            &[
                &(T::NS.0 as u64).to_be_bytes(),
                &T::id(&r).to_be_bytes(),
                &r.encode_to_vec(),
            ],
        );
    }
}

fn index_line<T: Table>(
    tx: &mut ReadTx<MemoryKv>,
    ix: &strims_dao::Index<T::Record>,
    key: &[u8],
    out: &mut Lines,
) {
    let mut ids = block(T::index_ids(tx, ix, key)).unwrap();
    ids.sort();
    let ids: Vec<u8> = ids.iter().flat_map(|id| id.to_be_bytes()).collect();
    out.put("index", &[&(ix.ns.0 as u64).to_be_bytes(), key, &ids]);
}

/// The ids of a chat server's records. Its icon's id is the server's.
#[derive(Clone, Copy, Default)]
struct ChatServerIds {
    server: u64,
    emote: u64,
    modifier: u64,
    tag: u64,
    profile: u64,
}

impl ChatServerIds {
    /// Each record as (namespace, id).
    fn records(&self) -> [(i64, u64); 6] {
        [
            (ChatServers::NS.0, self.server),
            (ChatServerIcons::NS.0, self.server),
            (ChatEmotes::NS.0, self.emote),
            (ChatModifiers::NS.0, self.modifier),
            (ChatTags::NS.0, self.tag),
            (ChatProfiles::NS.0, self.profile),
        ]
    }
}

/// Inserts a chat server named `name` on the network with `network_key`,
/// with an icon, an emote, a modifier, a tag and a muted profile of
/// `user`.
async fn insert_chat_server(
    tx: &mut WriteTx<MemoryKv>,
    network_key: &[u8],
    key: Key,
    name: &str,
    user: &[u8],
) -> ChatServerIds {
    let server = tx.generate_id().await.unwrap();
    ChatServers::insert(
        tx,
        Server {
            id: server,
            network_key: network_key.to_vec(),
            key: Some(key),
            room: Some(Room {
                name: name.into(),
                css: format!(".{name} {{}}"),
            }),
            admin_peer_keys: vec![user.to_vec()],
        },
    )
    .await
    .unwrap();
    ChatServerIcons::insert(
        tx,
        ServerIcon {
            id: server,
            image: Some(Image {
                r#type: ImageType::Png as i32,
                height: 1,
                width: 1,
                data: name.as_bytes().to_vec(),
            }),
        },
    )
    .await
    .unwrap();
    let emote = tx.generate_id().await.unwrap();
    ChatEmotes::insert(
        tx,
        Emote {
            id: emote,
            server_id: server,
            name: format!("{name}Emote"),
            images: vec![EmoteImage {
                data: name.as_bytes().to_vec(),
                file_type: EmoteFileType::FileTypePng as i32,
                height: 28,
                width: 28,
                ..Default::default()
            }],
            labels: vec!["label".into()],
            enable: true,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let modifier = tx.generate_id().await.unwrap();
    ChatModifiers::insert(
        tx,
        Modifier {
            id: modifier,
            server_id: server,
            name: "wide".into(),
            priority: 1,
            proc_chance: 0.5,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let tag = tx.generate_id().await.unwrap();
    ChatTags::insert(
        tx,
        Tag {
            id: tag,
            server_id: server,
            name: "nsfw".into(),
            color: "#ff0000".into(),
            sensitive: true,
        },
    )
    .await
    .unwrap();
    let profile = tx.generate_id().await.unwrap();
    ChatProfiles::insert(
        tx,
        ChatProfile {
            id: profile,
            server_id: server,
            peer_key: user.to_vec(),
            alias: "user".into(),
            mutes: vec![Mute {
                created_at: 100,
                duration_secs: 60,
                message: "spam".into(),
                moderator_peer_key: b"moderator".to_vec(),
            }],
            mute_deadline: 160,
        },
    )
    .await
    .unwrap();
    ChatServerIds {
        server,
        emote,
        modifier,
        tag,
        profile,
    }
}

#[test]
fn go_reads_a_store_this_crate_writes() {
    if !go_available() {
        return;
    }
    let now = Time::from_system(std::time::SystemTime::now());
    let kv = MemoryKv::new();
    let mut rng = det_rng(42);
    let (pk, store) = block(auth::sign_up(
        &kv,
        "bob",
        "correct horse",
        "linux",
        &mut rng,
        now,
    ))
    .unwrap();
    let session_key = StorageKey::from_bytes([77; 32]);
    let session = block(auth::create_session(&kv, &session_key, &pk, &mut rng, now)).unwrap();

    let rs = block(ReplicatedStore::new(store.clone())).unwrap();
    let mut events = rs.events().subscribe();
    let network_key = strims_crypto::generate_key(&mut rng);
    let doomed_key = strims_crypto::generate_key(&mut rng);
    let profile_key = block(async {
        let mut tx = store.read().await.unwrap();
        Profile::get(&mut tx).await.unwrap().unwrap().key.unwrap()
    });
    let owned_network = |id: u64, key: &Key, rng: &mut strims_base::rng::DetRng| {
        let usage = KeyUsage::Peer as i32 | KeyUsage::Sign as i32;
        let csr = strims_crypto::new_certificate_request(&profile_key, usage, "bob").unwrap();
        let cert = strims_crypto::sign_certificate_request_with_network(
            &csr,
            key,
            "net",
            strims_crypto::DEFAULT_CERT_TTL,
            now,
            rng,
        )
        .unwrap();
        Network {
            id,
            certificate: Some(cert),
            alias: "owned".into(),
            server_config: Some(ServerConfig {
                name: "net".into(),
                key: Some(key.clone()),
                ..Default::default()
            }),
            ..Default::default()
        }
    };
    let invitee = strims_crypto::generate_key(&mut rng);
    let swarm_key = strims_crypto::generate_key(&mut rng);
    let media = Listing {
        content: Some(directory_listing::Content::Media(
            directory_listing::Media {
                mime_type: "video/mp4".into(),
                swarm_uri: strims_ppspp::Uri::new(
                    strims_ppspp::SwarmId::new(&swarm_key.public),
                    strims_ppspp::UriOptions::default(),
                )
                .to_string(),
            },
        )),
    };
    let chat = Listing {
        content: Some(directory_listing::Content::Chat(directory_listing::Chat {
            key: invitee.public.clone(),
            name: "lobby".into(),
        })),
    };
    let (mut owned_id, mut doomed_id, mut owner_peer_id) = (0, 0, 0);
    let mut doomed_notification = 0;
    let mut chat_servers = [ChatServerIds::default(); 3];
    let chat_keys = [(); 3].map(|_| strims_crypto::generate_key(&mut rng));
    block(async {
        let mut tx = rs.write(now).await.unwrap();
        owned_id = tx.generate_id().await.unwrap();
        Networks::insert(&mut tx, owned_network(owned_id, &network_key, &mut rng))
            .await
            .unwrap();
        doomed_id = tx.generate_id().await.unwrap();
        Networks::insert(&mut tx, owned_network(doomed_id, &doomed_key, &mut rng))
            .await
            .unwrap();
        let id = tx.generate_id().await.unwrap();
        DirectoryListingRecords::insert(
            &mut tx,
            new_listing_record(id, doomed_id, media.clone(), None),
        )
        .await
        .unwrap();
        chat_servers[2] = insert_chat_server(
            &mut tx,
            &doomed_key.public,
            chat_keys[2].clone(),
            "doomed",
            &invitee.public,
        )
        .await;
        doomed_notification = tx.generate_id().await.unwrap();
        let n = new_notification(
            doomed_notification,
            now,
            Status::Success,
            "Certificate renewed",
            "",
            Some(network_subject(doomed_id)),
        );
        dispatch(&mut tx, n).await.unwrap();
        Networks::delete(&mut tx, doomed_id).await.unwrap();
        let moderated = ListingModeration {
            is_banned: Some(true),
            category: Some("games".into()),
            ..Default::default()
        };
        for (listing, moderation) in [(media.clone(), Some(moderated)), (chat.clone(), None)] {
            let id = tx.generate_id().await.unwrap();
            DirectoryListingRecords::insert(
                &mut tx,
                new_listing_record(id, owned_id, listing, moderation),
            )
            .await
            .unwrap();
        }
        let id = tx.generate_id().await.unwrap();
        let moderator = UserModeration {
            is_moderator: Some(true),
            disable_publish: Some(true),
            ..Default::default()
        };
        DirectoryUserRecords::insert(
            &mut tx,
            new_user_record(id, owned_id, &invitee.public, moderator),
        )
        .await
        .unwrap();
        owner_peer_id = NetworkPeers::index_get(
            &mut tx,
            &NetworkPeers::BY_PUBLIC_KEY,
            &network_scoped_key(owned_id, &profile_key.public),
        )
        .await
        .unwrap()
        .unwrap()
        .id;
        let id = tx.generate_id().await.unwrap();
        NetworkPeers::insert(
            &mut tx,
            new_network_peer(id, owned_id, &invitee.public, "invitee", owner_peer_id, now),
        )
        .await
        .unwrap();
        let id = tx.generate_id().await.unwrap();
        AliasReservations::insert(
            &mut tx,
            new_alias_reservation(id, owned_id, "invitee", &invitee.public),
        )
        .await
        .unwrap();
        NetworkUiConfig::set(
            &mut tx,
            NetworkUiConfigRecord {
                network_display_order: vec![owned_id],
            },
        )
        .await
        .unwrap();
        for url in ["wss://a.example/ws", "wss://b.example/ws"] {
            let id = tx.generate_id().await.unwrap();
            BootstrapClients::insert(
                &mut tx,
                BootstrapClient {
                    id,
                    client_options: Some(bootstrap_client::ClientOptions::WebsocketOptions(
                        BootstrapClientWebSocketOptions {
                            url: url.into(),
                            insecure_skip_verify_tls: false,
                        },
                    )),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        }
        ChatUiConfig::set(
            &mut tx,
            UiConfig {
                replicate: true,
                show_time: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();

        let mut tx = rs.write(now).await.unwrap();
        let mut highlights = vec![];
        for (alias, key) in [("one", b"peer-1"), ("two", b"peer-2")] {
            let id = tx.generate_id().await.unwrap();
            highlights.push(id);
            ChatUiConfigHighlights::insert(
                &mut tx,
                UiConfigHighlight {
                    id,
                    alias: alias.into(),
                    peer_key: key.to_vec(),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        }
        ChatUiConfigHighlights::delete(&mut tx, highlights[1])
            .await
            .unwrap();
        let id = tx.generate_id().await.unwrap();
        ChatUiConfigTags::insert(
            &mut tx,
            UiConfigTag {
                id,
                alias: "tag".into(),
                peer_key: b"peer-3".to_vec(),
                color: "#00ff00".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let id = tx.generate_id().await.unwrap();
        ChatUiConfigIgnores::insert(
            &mut tx,
            UiConfigIgnore {
                id,
                alias: "ignored".into(),
                peer_key: b"peer-4".to_vec(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        for last in [10, 20] {
            let id = tx.generate_id().await.unwrap();
            WhisperThreads::insert(
                &mut tx,
                WhisperThread {
                    id,
                    peer_key: b"peer-5".to_vec(),
                    alias: format!("at {last}"),
                    last_message_time: last,
                    has_unread: last == 10,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        }
        for (state, peer_message_id) in [
            (MessageState::Unread, 5),
            (MessageState::Enqueued, 0),
            (MessageState::Read, 6),
        ] {
            let id = tx.generate_id().await.unwrap();
            WhisperRecords::insert(
                &mut tx,
                WhisperRecord {
                    id,
                    peer_key: b"peer-5".to_vec(),
                    state: state as i32,
                    peer_message_id,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        }
        let id = tx.generate_id().await.unwrap();
        Devices::insert(
            &mut tx,
            Device {
                id,
                device: "phone".into(),
                os: "android".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        for (i, name) in ["kept", "deleted"].into_iter().enumerate() {
            chat_servers[i] = insert_chat_server(
                &mut tx,
                &network_key.public,
                chat_keys[i].clone(),
                name,
                &invitee.public,
            )
            .await;
        }
        assert!(
            ChatServers::delete(&mut tx, chat_servers[1].server)
                .await
                .unwrap()
        );
        let id = tx.generate_id().await.unwrap();
        let mut failed = new_notification(
            id,
            now,
            Status::Error,
            "Certificate renewal failed",
            "rate limited",
            Some(network_subject(owned_id)),
        );
        failed.error_code = 2;
        dispatch(&mut tx, failed).await.unwrap();
        tx.commit().await.unwrap();
    });
    let mut token = [0u8; 16];
    token[0] = 1;
    block(lease::try_lock(
        rs.store(),
        &lease::key(&["replication"]),
        &token,
        now,
    ))
    .unwrap();

    let mut logs: Vec<EventLog> = vec![];
    while let Some(Some(Recv::Event(e))) = events.next().now_or_never() {
        if let StoreEvent::EventLog(l) = Rc::unwrap_or_clone(e) {
            logs.push(l);
        }
    }
    assert_eq!(logs.len(), 2);

    let mut out = Lines(String::new());
    for (table, entries) in kv.dump() {
        for (k, v) in entries {
            out.put("kv", &[table.as_bytes(), k.as_bytes(), &v]);
        }
    }
    out.put(
        "auth",
        &[b"bob", b"correct horse", &pk.id.to_be_bytes(), &pk.key],
    );
    out.put(
        "session",
        &[
            session_key.bytes(),
            &session.eol.to_be_bytes(),
            &session.token,
            &pk.id.to_be_bytes(),
            &pk.key,
        ],
    );

    let mut tx = block(store.read()).unwrap();
    let profile = block(Profile::get(&mut tx)).unwrap().unwrap();
    out.put(
        "record",
        &[
            &2001u64.to_be_bytes(),
            &0u64.to_be_bytes(),
            &profile.encode_to_vec(),
        ],
    );
    let ranges = block(ProfileIds::get(&mut tx)).unwrap().unwrap();
    out.put(
        "record",
        &[
            &2002u64.to_be_bytes(),
            &0u64.to_be_bytes(),
            &ranges.encode_to_vec(),
        ],
    );
    let ui = block(ChatUiConfig::get(&mut tx)).unwrap().unwrap();
    out.put(
        "record",
        &[
            &6008u64.to_be_bytes(),
            &0u64.to_be_bytes(),
            &ui.encode_to_vec(),
        ],
    );
    let network_ui = block(NetworkUiConfig::get(&mut tx)).unwrap().unwrap();
    out.put(
        "record",
        &[
            &4008u64.to_be_bytes(),
            &0u64.to_be_bytes(),
            &network_ui.encode_to_vec(),
        ],
    );
    record_lines::<CertificateLogs>(&mut tx, &mut out);
    record_lines::<NetworkPeers>(&mut tx, &mut out);
    record_lines::<AliasReservations>(&mut tx, &mut out);
    record_lines::<Devices>(&mut tx, &mut out);
    record_lines::<Networks>(&mut tx, &mut out);
    record_lines::<BootstrapClients>(&mut tx, &mut out);
    record_lines::<ChatUiConfigHighlights>(&mut tx, &mut out);
    record_lines::<ChatUiConfigTags>(&mut tx, &mut out);
    record_lines::<ChatUiConfigIgnores>(&mut tx, &mut out);
    record_lines::<WhisperThreads>(&mut tx, &mut out);
    record_lines::<WhisperRecords>(&mut tx, &mut out);
    record_lines::<EventLogs>(&mut tx, &mut out);
    record_lines::<Checkpoints>(&mut tx, &mut out);
    record_lines::<DirectoryListingRecords>(&mut tx, &mut out);
    record_lines::<DirectoryUserRecords>(&mut tx, &mut out);
    record_lines::<ChatServers>(&mut tx, &mut out);
    record_lines::<ChatServerIcons>(&mut tx, &mut out);
    record_lines::<ChatEmotes>(&mut tx, &mut out);
    record_lines::<ChatModifiers>(&mut tx, &mut out);
    record_lines::<ChatTags>(&mut tx, &mut out);
    record_lines::<ChatProfiles>(&mut tx, &mut out);
    record_lines::<Notifications>(&mut tx, &mut out);

    index_line::<Networks>(&mut tx, &Networks::BY_KEY, &network_key.public, &mut out);
    let scoped = |b: &[u8]| network_scoped_key(owned_id, b);
    let cert_logs = block(CertificateLogs::get_all(&mut tx)).unwrap();
    assert_eq!(
        cert_logs.len(),
        2,
        "the owned network's chain; the deleted one's went"
    );
    for log in &cert_logs {
        let c = log.certificate.as_ref().unwrap();
        index_line::<CertificateLogs>(
            &mut tx,
            &CertificateLogs::BY_SERIAL_NUMBER,
            &scoped(&c.serial_number),
            &mut out,
        );
        index_line::<CertificateLogs>(
            &mut tx,
            &CertificateLogs::BY_SUBJECT,
            &scoped(c.subject.as_bytes()),
            &mut out,
        );
        index_line::<CertificateLogs>(&mut tx, &CertificateLogs::BY_KEY, &scoped(&c.key), &mut out);
    }
    for id in [owned_id, doomed_id] {
        index_line::<CertificateLogs>(
            &mut tx,
            &CertificateLogs::BY_NETWORK,
            &ref_key(id),
            &mut out,
        );
        index_line::<NetworkPeers>(&mut tx, &NetworkPeers::BY_NETWORK, &ref_key(id), &mut out);
        index_line::<AliasReservations>(
            &mut tx,
            &AliasReservations::BY_NETWORK,
            &ref_key(id),
            &mut out,
        );
        index_line::<DirectoryListingRecords>(
            &mut tx,
            &DirectoryListingRecords::BY_NETWORK,
            &ref_key(id),
            &mut out,
        );
        index_line::<DirectoryUserRecords>(
            &mut tx,
            &DirectoryUserRecords::BY_NETWORK,
            &ref_key(id),
            &mut out,
        );
        for listing in [&media, &chat] {
            index_line::<DirectoryListingRecords>(
                &mut tx,
                &DirectoryListingRecords::BY_LISTING,
                &listing_key(id, Some(listing)),
                &mut out,
            );
        }
        for key in [&profile_key.public, &invitee.public] {
            index_line::<DirectoryUserRecords>(
                &mut tx,
                &DirectoryUserRecords::BY_PEER_KEY,
                &network_scoped_key(id, key),
                &mut out,
            );
        }
    }
    let users = block(DirectoryUserRecords::get_all(&mut tx)).unwrap();
    assert_eq!(
        users.len(),
        2,
        "the owned network's admin and moderator; the deleted one's admin went"
    );
    let listings = block(DirectoryListingRecords::get_all(&mut tx)).unwrap();
    assert!(listings.iter().all(|r| r.network_id == owned_id));
    assert_eq!(listings.len(), 2);
    index_line::<NetworkPeers>(
        &mut tx,
        &NetworkPeers::BY_PUBLIC_KEY,
        &scoped(&invitee.public),
        &mut out,
    );
    index_line::<NetworkPeers>(
        &mut tx,
        &NetworkPeers::BY_INVITER,
        &ref_key(owner_peer_id),
        &mut out,
    );
    index_line::<AliasReservations>(
        &mut tx,
        &AliasReservations::BY_ALIAS,
        &scoped(b"bob"),
        &mut out,
    );
    index_line::<BootstrapClients>(
        &mut tx,
        &BootstrapClients::BY_CLIENT_OPTIONS,
        b"wss://b.example/ws",
        &mut out,
    );
    index_line::<ChatUiConfigHighlights>(
        &mut tx,
        &ChatUiConfigHighlights::BY_PEER_KEY,
        b"peer-1",
        &mut out,
    );
    index_line::<ChatUiConfigHighlights>(
        &mut tx,
        &ChatUiConfigHighlights::BY_PEER_KEY,
        b"peer-2",
        &mut out,
    );
    index_line::<ChatUiConfigTags>(&mut tx, &ChatUiConfigTags::BY_PEER_KEY, b"peer-3", &mut out);
    index_line::<ChatUiConfigIgnores>(
        &mut tx,
        &ChatUiConfigIgnores::BY_PEER_KEY,
        b"peer-4",
        &mut out,
    );
    index_line::<WhisperThreads>(&mut tx, &WhisperThreads::BY_PEER_KEY, b"peer-5", &mut out);
    index_line::<WhisperRecords>(&mut tx, &WhisperRecords::BY_PEER_KEY, b"peer-5", &mut out);
    index_line::<WhisperRecords>(
        &mut tx,
        &WhisperRecords::UNREAD_BY_PEER_KEY,
        b"peer-5",
        &mut out,
    );
    index_line::<WhisperRecords>(
        &mut tx,
        &WhisperRecords::BY_STATE,
        &state_key(MessageState::Read as i32),
        &mut out,
    );
    for ids in &chat_servers {
        let server = ref_key(ids.server);
        index_line::<ChatEmotes>(&mut tx, &ChatEmotes::BY_SERVER, &server, &mut out);
        index_line::<ChatModifiers>(&mut tx, &ChatModifiers::BY_SERVER, &server, &mut out);
        index_line::<ChatTags>(&mut tx, &ChatTags::BY_SERVER, &server, &mut out);
        index_line::<ChatProfiles>(&mut tx, &ChatProfiles::BY_SERVER, &server, &mut out);
        index_line::<ChatProfiles>(
            &mut tx,
            &ChatProfiles::BY_PEER_KEY,
            &profile_peer_key(ids.server, &invitee.public),
            &mut out,
        );
    }
    // The deleted server and the one on the deleted network left nothing.
    for (ns, id) in chat_servers[1]
        .records()
        .into_iter()
        .chain(chat_servers[2].records())
    {
        out.put("absent", &[&(ns as u64).to_be_bytes(), &id.to_be_bytes()]);
    }
    out.put(
        "absent",
        &[
            &(Notifications::NS.0 as u64).to_be_bytes(),
            &doomed_notification.to_be_bytes(),
        ],
    );
    assert_eq!(block(Notifications::get_all(&mut tx)).unwrap().len(), 1);
    let servers = block(ChatServers::get_all(&mut tx)).unwrap();
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].id, chat_servers[0].server);
    assert_eq!(block(ChatServerIcons::get_all(&mut tx)).unwrap().len(), 1);
    let emotes = block(ChatEmotes::get_all(&mut tx)).unwrap();
    assert!(emotes.len() == 1 && emotes[0].server_id == chat_servers[0].server);
    let profiles = block(ChatProfiles::get_all(&mut tx)).unwrap();
    assert!(profiles.len() == 1 && profiles[0].server_id == chat_servers[0].server);
    let networks = block(Networks::get_all(&mut tx)).unwrap();
    let bootstraps = block(BootstrapClients::get_all(&mut tx)).unwrap();
    let devices = block(Devices::get_all(&mut tx)).unwrap();
    drop(tx);

    let mut applied: Vec<Vec<u8>> = vec![0xbeefu64.to_be_bytes().to_vec()];
    applied.extend(logs.iter().map(|l| l.encode_to_vec()));
    out.put(
        "apply",
        &applied.iter().map(Vec::as_slice).collect::<Vec<_>>(),
    );

    let new_device = 0x1234u64;
    let pairing = PairingToken {
        auth: block(auth::get_user(&kv, "bob")).unwrap(),
        profile: Some(strims_proto::profile::v1::Profile {
            device_id: new_device,
            ..profile
        }),
        networks: networks
            .into_iter()
            .map(|n| Network {
                server_config: None,
                ..n
            })
            .collect(),
        bootstraps,
        devices,
        profile_id: block(allocate_ids(rs.store(), 1000, now)).unwrap(),
    };
    out.put(
        "pairing",
        &[&pairing.encode_to_vec(), &pk.key, b"correct horse"],
    );

    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("strims-dao-cross-go.hex");
    std::fs::write(&path, out.0).unwrap();
    let run = Command::new("go")
        .args([
            "run",
            "-tags",
            "golden",
            "./crates/strims-dao/golden",
            "-verify",
        ])
        .arg(&path)
        .current_dir(repo_root())
        .output()
        .expect("run go");
    let stdout = String::from_utf8_lossy(&run.stdout);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        run.status.success() && stdout.trim() == "ok",
        "go verify failed:\n{stdout}\n{stderr}"
    );

    // The pairing token this test sends is one this crate imports too.
    let paired = MemoryKv::new();
    let paired_store = block(auth::import_pairing_token(
        &paired,
        &pairing,
        &ProfileKey { ..pk.clone() },
        &mut rng,
        now,
    ))
    .unwrap();
    let mut tx = block(paired_store.read()).unwrap();
    assert_eq!(
        block(Profile::get(&mut tx)).unwrap().unwrap().device_id,
        new_device
    );
}
