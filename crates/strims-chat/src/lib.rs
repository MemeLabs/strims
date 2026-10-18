// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The chat service.
//!
//! A chat server runs each message it accepts through the message text
//! pipeline:
//! - [`text`] parses the body into spans, emotes with their modifiers, tags
//!   and nick mentions;
//! - [`entities`] extracts the message's entities (links, emoji and the
//!   parsed nodes) with rune bounds, and adds the server's internal emote
//!   modifiers;
//! - [`combo`] marks runs of the same lone emote.
//!
//! [`Server`] is the sans-IO state of a server: sends with their bounds
//! and send rates ([`rate`]), moderation and mutes, and the nicks of its
//! directory listing. [`duration`] parses the duration strings mutes and
//! ignores are given in.
//!
//! A server publishes its room, icon, emotes, modifiers and tags as
//! [`assets`] bundles: a unified bundle after each stream reset, then
//! deltas. A client decodes the event and asset records through the
//! bounded readers in [`reader`].
//!
//! [`whisper`] decides which whispers a profile takes, and schedules the
//! delivery of those it sends.
//!
//! [`ChatControl`] runs the chat servers the profile hosts and the rooms
//! the node reads, around these sans-IO parts; [`WhisperControl`] runs
//! the profile's whispers.
//!
//! # Wire contract
//!
//! - **Swarms.** A server's event and asset swarms have the server's
//!   public key as their swarm id, the options [`event_options`] and
//!   [`asset_options`], and the transfer salts [`EVENT_SALT`] and
//!   [`ASSET_SALT`]. The event swarm carries `ServerEvent` records in
//!   frames of [`EVENT_FRAME`] bytes, the asset swarm `AssetBundle`
//!   records in frames of [`ASSET_FRAME`] bytes: a unified bundle after
//!   each stream reset, then deltas ([`assets`]). Readers start at the
//!   swarms' window tails, so a reader takes the asset stream from its
//!   unified bundle on and the event stream with its recent history.
//! - **Addresses.** A server publishes its address under (server key,
//!   [`ADDRESS_SALT`]), signed by the server key, on a vpn port of its
//!   own, and serves `chat.v1.Chat` there; a profile publishes its whisper
//!   address under (profile key, [`WHISPER_SALT`]).
//! - **Directory.** A served server publishes the listing `Chat{key: server
//!   key, name: room name}` on its network's directory.
//! - **Whispers.** A profile serves `chat.v1.Whisper.SendMessage` at its
//!   whisper address. The caller's vpn certificate names the sender: its
//!   membership certificate's key and subject, under the network's root.
//!   A whisper's `id` is the sender's record id, the same on every
//!   attempt, and a receiver stores one whisper per (`id`, sender).
//! - **Errors.** A refused call is answered with the message of its
//!   [`ChatError`]; a refused whisper with [`whisper::WHISPER_REFUSED`],
//!   and one over its sender's rate with [`ChatError::TooFast`]'s.
//!
//! # Leases
//!
//! Among the processes sharing a store, the holder of the lease
//! [`lease_key`] of a server's id serves that server, and the holder of
//! [`whisper::lease_key`] runs the profile's whispers.
//!
//! # Running
//!
//! [`ChatControl`] runs on the node the driver supplies through
//! [`strims_directory::Host`], beside the node's
//! [`NetworkControl`](strims_network::NetworkControl),
//! [`TransferControl`](strims_transfer::TransferControl) and
//! [`DirectoryControl`](strims_directory::DirectoryControl). The driver
//! runs [`ChatControl::run`] as a local task. The control follows the
//! profile's `ChatServers` records in the store, the networks the node
//! joins and the directories' events:
//!
//! - Each server the profile owns is served on its record's network by
//!   the process holding its lease, once the node has joined the network
//!   and the profile's membership is trusted. The server takes its calls,
//!   writes their events, publishes its assets a debounce
//!   ([`Config::asset_debounce`]) after they change in the store, and
//!   keeps its directory listing published; its nicks are the listing's
//!   users. A record moved to another network stops the server and starts
//!   it on the new network once the old one's swarms are gone.
//! - [`ChatControl::open_client`] opens a client stream of any server
//!   ([`ClientStream`]). A server this process doesn't serve is read
//!   through its swarms, added on the network while streams are open and
//!   for [`Config::linger`] after the last. The asset swarm of a read
//!   server starts from the cache this device last stored of it, so a
//!   reopened room has its bundles with no server or peers to serve them.
//! - Calls ([`ChatControl::send_message`], [`ChatControl::mute`] and the
//!   rest) go to a server this process serves directly, as its host
//!   certificate, and to any other from the network's member port, a vpn
//!   port opened once the network is joined and the membership trusted.
//!
//! What changes is reported as [`ChatEvent`]s; [`ChatControl::servers`]
//! reads the state at any time. [`bind_frontend`] serves the UI's chat
//! calls on the chat and whisper controls ([`frontend`]).
//!
//! # Whispers
//!
//! [`WhisperControl`] runs a profile's whispers on a node, beside its
//! [`NetworkControl`](strims_network::NetworkControl). The driver runs
//! [`WhisperControl::run`] as a local task.
//!
//! - It serves `chat.v1.Whisper` on a whisper port of every network the
//!   node has joined with a trusted membership.
//! - Among the processes sharing the profile's store, the holder of the
//!   lease [`whisper::lease_key`] publishes those ports' addresses and
//!   delivers the profile's whispers.
//! - A received whisper is stored UNREAD with its sender's thread marked
//!   unread, unless the sender is ignored or over a bound
//!   ([`whisper::check_receive`]). A whisper received again is taken
//!   without a write.
//! - [`WhisperControl::enqueue`] stores a whisper ENQUEUED. Delivery
//!   retries on the [`whisper::Deliveries`] schedule until the recipient
//!   takes it (DELIVERED) or refuses it, or the attempts or time run out
//!   (FAILED). A state is written only over ENQUEUED, so a record the user
//!   deleted or changed meanwhile keeps its own.
//!
//! # Bounds
//!
//! Callers, servers, senders and their records may be hostile.
//!
//! - A served server bounds what a call can make it hold with
//!   [`ServerLimits`] and the constants of [`server`]; a refused call fails
//!   with a [`ChatError`].
//! - A client stream bounds what a server's records can make it decode and
//!   hold with [`reader::RoomLimits`], and drops and counts the rest
//!   ([`ClientMetrics`]).
//! - The control runs at most [`Config::max_served`] servers and reads at
//!   most [`Config::max_rooms`] other rooms, with at most
//!   [`Config::max_streams_per_room`] streams each; past those, a record is
//!   left unserved and a stream is refused.
//! - [`WhisperLimits`](whisper::WhisperLimits) bounds what a sender can
//!   make a profile store (body bytes, unread whispers per sender, threads)
//!   and the delivery queue (records pending and attempts in flight;
//!   [`WhisperControl::enqueue`] refuses past the pending bound). Each
//!   sender's calls are rate limited by [`WhisperConfig::rate`], over at
//!   most [`RateConfig::max_keys`](rate::RateConfig::max_keys) senders,
//!   and each delivery attempt ends within
//!   [`WhisperConfig::attempt_deadline`].

pub mod assets;
mod client;
pub mod combo;
mod config;
mod control;
pub mod duration;
pub mod entities;
mod error;
mod events;
pub mod frontend;
pub mod rate;
pub mod reader;
mod runner;
pub mod server;
mod services;
pub mod swarms;
pub mod text;
pub mod whisper;

pub use client::{ClientMetrics, ClientStream};
pub use config::{Config, MAX_ROOMS, MAX_SERVED, MAX_STREAMS_PER_ROOM, Metrics};
pub use control::{ChatControl, ChatFailure, ChatServerStatus, ServerMode};
pub use error::ChatError;
pub use events::{ChatEvent, ClientEvent};
pub use frontend::{ChatFrontend, bind_frontend};
pub use runner::lease_key;
pub use server::{Server, ServerAction, ServerLimits, ServerMetrics};
pub use swarms::{
    ADDRESS_SALT, ASSET_FRAME, ASSET_SALT, ASSET_WINDOW_BYTES, EVENT_FRAME, EVENT_SALT,
    WHISPER_SALT, asset_options, event_options,
};
pub use whisper::{EnqueueError, WhisperConfig, WhisperControl, WhisperMetrics};
