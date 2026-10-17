// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A chat server's state: its record, the names its messages are parsed
//! against, the running combo, send rates and active mutes. Sans-IO: the
//! control feeds it the Chat service's calls with the profiles it read,
//! the server's assets and the users of its directory listing, and
//! carries out its [`ServerAction`]s, draining them after every call.
//!
//! # Messages
//!
//! [`Server::send_message`] refuses, in order: a caller or body over its
//! bound, a caller over its send rate, a muted caller and a repeat in the
//! running combo. A refused send queues nothing; one refused for a mute
//! or a combo repeat has spent from the caller's send rate. An accepted
//! message carries the server time in unix milliseconds, the caller's
//! peer key and alias as its nick, the body, and the entities [`extract`]
//! finds, with the server's internal modifiers added and the combo
//! applied; it goes out as [`ServerAction::Event`].
//!
//! # Moderation
//!
//! The moderators are the profile hosting the server (its owner peer key)
//! and the record's `admin_peer_keys`. Mute deadlines are unix seconds; a
//! user is muted while its deadline is after now. Profiles are written
//! only by moderation: a mute sets the deadline to now plus its duration
//! and, when it has a message, appends it to the profile's mutes, creating
//! the profile of a user without one; an unmute sets an existing profile's
//! deadline to now. Each queues the profile to persist as
//! [`ServerAction::SaveProfile`], and takes effect at once: the server
//! holds the active mutes in memory, loaded with [`Server::set_mutes`].
//!
//! # Nicks
//!
//! Nick mentions match the users of the server's directory listing:
//! [`Server::reset_nicks`] replaces them, [`Server::nick_changed`] follows
//! the listing's user events.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};
use strims_directory::view::User;
use strims_directory::{Caller, MAX_ALIAS_BYTES, MAX_PEER_KEY_BYTES, UserEventKind};
use strims_proto::chat::v1::{
    Emote, GetMuteResponse, Message, Modifier, MuteRequest, Profile, ServerEvent, Tag,
    UnmuteRequest, profile, server_event,
};

use crate::ChatError;
use crate::combo::Combos;
use crate::entities::{Patterns, add_internal_modifiers, extract};
use crate::rate::{RateConfig, RateLimiter};
use crate::text::{MAX_NICKS, ParserContext};

/// The longest message body, in bytes.
pub const MAX_BODY_BYTES: usize = 2048;
/// The longest mute message, in bytes.
pub const MAX_MUTE_MESSAGE_BYTES: usize = 512;
/// The most mutes a profile keeps; a new one beyond it drops the oldest.
pub const MAX_PROFILE_MUTES: usize = 32;
/// The most active mutes a server holds by default.
pub const MAX_MUTES: usize = 4096;

/// What a [`Server`] takes and holds at most.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServerLimits {
    /// The longest body; a longer one fails with
    /// [`ChatError::MessageTooLong`].
    pub max_body_bytes: usize,
    /// Each peer key's send rate, and the keys tracked. A send over it
    /// fails with [`ChatError::TooFast`].
    pub send_rate: RateConfig,
    /// Active mutes. Beyond it a new mute evicts the one that ends first,
    /// and [`Server::set_mutes`] keeps those that end last.
    pub max_mutes: usize,
}

impl Default for ServerLimits {
    fn default() -> ServerLimits {
        ServerLimits {
            max_body_bytes: MAX_BODY_BYTES,
            send_rate: RateConfig::default(),
            max_mutes: MAX_MUTES,
        }
    }
}

/// What the control must do.
#[derive(Clone, Debug, PartialEq)]
// Nearly every action is an event.
#[allow(clippy::large_enum_variant)]
pub enum ServerAction {
    /// Write the event to the event swarm.
    Event(ServerEvent),
    /// Store the profile, replacing the stored one with its id.
    SaveProfile(Profile),
}

/// What a [`Server`] has done.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServerMetrics {
    /// Messages sent, and those among them that continued a combo.
    pub messages: u64,
    pub combo_messages: u64,
    /// Sends refused, by reason.
    pub refused_too_large: u64,
    pub refused_too_long: u64,
    pub refused_too_fast: u64,
    pub refused_muted: u64,
    pub refused_combo: u64,
    /// Mutes and unmutes applied, and those refused.
    pub mutes: u64,
    pub unmutes: u64,
    pub refused_moderation: u64,
    /// Active mutes dropped for [`ServerLimits::max_mutes`].
    pub mutes_dropped: u64,
    /// Listing users left out of the nicks for their bound.
    pub nicks_dropped: u64,
}

/// An active mute.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Mute {
    /// Unix seconds.
    deadline: i64,
    /// The message of the mute that set the deadline; empty without one.
    message: String,
}

/// A chat server's sans-IO state; see the module docs.
///
/// Calls that parse a message take the control's [`Patterns`]: they are
/// costly to compile and hold several MiB, and one set serves every
/// server a control runs.
pub struct Server {
    record: strims_proto::chat::v1::Server,
    owner_peer_key: Vec<u8>,
    limits: ServerLimits,
    rng: DetRng,
    ctx: ParserContext,
    /// The modifiers [`add_internal_modifiers`] adds, from the server's own
    /// assets.
    internal_modifiers: Vec<Modifier>,
    combos: Combos,
    rate: RateLimiter,
    /// Active mutes by peer key; at most [`ServerLimits::max_mutes`].
    mutes: BTreeMap<Vec<u8>, Mute>,
    /// `mutes` by deadline.
    mutes_by_deadline: BTreeSet<(i64, Vec<u8>)>,
    /// The alias each nick of `ctx` was added under, by peer key; at most
    /// [`MAX_NICKS`].
    aliases: BTreeMap<Vec<u8>, String>,
    /// Drained by the control after every call, which adds at most one.
    actions: VecDeque<ServerAction>,
    metrics: ServerMetrics,
}

impl Server {
    /// The server of `record`, hosted by the profile with
    /// `owner_peer_key`, with no assets, nicks or mutes.
    pub fn new(
        record: &strims_proto::chat::v1::Server,
        owner_peer_key: &[u8],
        limits: ServerLimits,
        rng: DetRng,
    ) -> Server {
        Server {
            record: record.clone(),
            owner_peer_key: owner_peer_key.to_vec(),
            limits,
            rng,
            ctx: ParserContext::new(),
            internal_modifiers: Vec::new(),
            combos: Combos::new(),
            rate: RateLimiter::new(limits.send_rate),
            mutes: BTreeMap::new(),
            mutes_by_deadline: BTreeSet::new(),
            aliases: BTreeMap::new(),
            actions: VecDeque::new(),
            metrics: ServerMetrics::default(),
        }
    }

    pub fn record(&self) -> &strims_proto::chat::v1::Server {
        &self.record
    }

    /// Takes the server's changed record: its room and admins.
    pub fn set_record(&mut self, record: &strims_proto::chat::v1::Server) {
        self.record.clone_from(record);
    }

    /// Replaces the names messages are parsed against: the enabled
    /// emotes, the modifiers that aren't internal, and the tags. The
    /// internal modifiers are kept, in order, to add at random.
    pub fn sync_assets(&mut self, emotes: &[Emote], modifiers: &[Modifier], tags: &[Tag]) {
        self.ctx
            .set_emotes(emotes.iter().filter(|e| e.enable).map(|e| e.name.as_str()));
        self.ctx.set_modifiers(
            modifiers
                .iter()
                .filter(|m| !m.internal)
                .map(|m| m.name.as_str()),
        );
        self.ctx.set_tags(tags.iter().map(|t| t.name.as_str()));
        self.internal_modifiers = modifiers.iter().filter(|m| m.internal).cloned().collect();
    }

    /// The names messages are parsed against.
    pub fn parser_context(&self) -> &ParserContext {
        &self.ctx
    }

    /// Whether `peer_key` may mute and unmute: the owner and the admins.
    pub fn is_moderator(&self, peer_key: &[u8]) -> bool {
        peer_key == self.owner_peer_key.as_slice()
            || self.record.admin_peer_keys.iter().any(|k| k == peer_key)
    }

    /// Sends `body` as `caller`; see the module docs.
    pub fn send_message(
        &mut self,
        now: Time,
        caller: &Caller,
        body: &str,
        patterns: &mut Patterns,
    ) -> Result<(), ChatError> {
        let res = self.try_send(now, caller, body, patterns);
        if let Err(e) = res {
            let m = &mut self.metrics;
            match e {
                ChatError::TooLarge => m.refused_too_large += 1,
                ChatError::MessageTooLong => m.refused_too_long += 1,
                ChatError::TooFast => m.refused_too_fast += 1,
                ChatError::Muted => m.refused_muted += 1,
                ChatError::ComboDuplicate => m.refused_combo += 1,
                ChatError::NotModerator => {}
            }
        }
        res
    }

    fn try_send(
        &mut self,
        now: Time,
        caller: &Caller,
        body: &str,
        patterns: &mut Patterns,
    ) -> Result<(), ChatError> {
        check_caller(caller)?;
        if body.len() > self.limits.max_body_bytes {
            return Err(ChatError::MessageTooLong);
        }
        if !self.rate.try_take(now, &caller.peer_key) {
            return Err(ChatError::TooFast);
        }
        if self.muted(now, &caller.peer_key).is_some() {
            return Err(ChatError::Muted);
        }

        let mut entities = extract(body, &self.ctx, patterns);
        add_internal_modifiers(&mut entities, &self.internal_modifiers, &mut self.rng);
        self.combos
            .transform(&caller.peer_key, body, &mut entities)?;
        if entities.emotes.first().is_some_and(|e| e.combo > 0) {
            self.metrics.combo_messages += 1;
        }

        self.metrics.messages += 1;
        self.actions.push_back(ServerAction::Event(ServerEvent {
            body: Some(server_event::Body::Message(Message {
                server_time: now.unix_millis(),
                peer_key: caller.peer_key.clone(),
                nick: caller.alias.clone(),
                body: body.to_owned(),
                entities: Some(entities),
                viewed_listing: None,
            })),
        }));
        Ok(())
    }

    /// Checks that `moderator` may moderate `target_peer_key`, as
    /// [`Server::mute`] and [`Server::unmute`] do first. The control
    /// checks before it reads the target's profile.
    pub fn check_moderation(
        &self,
        moderator: &Caller,
        target_peer_key: &[u8],
    ) -> Result<(), ChatError> {
        check_caller(moderator)?;
        if target_peer_key.len() > MAX_PEER_KEY_BYTES {
            return Err(ChatError::TooLarge);
        }
        if !self.is_moderator(&moderator.peer_key) {
            return Err(ChatError::NotModerator);
        }
        Ok(())
    }

    /// Mutes `req.peer_key` for `req.duration_secs` from `now`, as
    /// `moderator`. `existing` is the target's stored profile; without one,
    /// a new profile with id `new_id` and the target's listing alias is
    /// created. Queues the profile to persist.
    ///
    /// `existing` must include the profiles this server queued earlier,
    /// or the mutes they added are lost: the control reads it after
    /// storing them.
    pub fn mute(
        &mut self,
        now: Time,
        moderator: &Caller,
        req: &MuteRequest,
        existing: Option<Profile>,
        new_id: u64,
    ) -> Result<(), ChatError> {
        let checked = match self.check_moderation(moderator, &req.peer_key) {
            Ok(()) if req.message.len() > MAX_MUTE_MESSAGE_BYTES => Err(ChatError::TooLarge),
            res => res,
        };
        if let Err(e) = checked {
            self.metrics.refused_moderation += 1;
            return Err(e);
        }

        let mut profile = existing.unwrap_or_else(|| Profile {
            id: new_id,
            server_id: self.record.id,
            peer_key: req.peer_key.clone(),
            alias: self.aliases.get(&req.peer_key).cloned().unwrap_or_default(),
            mutes: Vec::new(),
            mute_deadline: 0,
        });
        let deadline = (now + Duration::from_secs(i64::from(req.duration_secs))).unix();
        profile.mute_deadline = deadline;
        if !req.message.is_empty() {
            profile.mutes.push(profile::Mute {
                created_at: now.unix(),
                duration_secs: req.duration_secs,
                message: req.message.clone(),
                moderator_peer_key: moderator.peer_key.clone(),
            });
        }
        let excess = profile.mutes.len().saturating_sub(MAX_PROFILE_MUTES);
        profile.mutes.drain(..excess);

        self.remove_mute(&req.peer_key);
        if active(deadline, now) {
            self.prune_mutes(now);
            while self.mutes.len() >= self.limits.max_mutes.max(1) {
                let Some((_, k)) = self.mutes_by_deadline.pop_first() else {
                    break;
                };
                self.mutes.remove(&k);
                self.metrics.mutes_dropped += 1;
            }
            self.insert_mute(
                req.peer_key.clone(),
                Mute {
                    deadline,
                    message: req.message.clone(),
                },
            );
        }

        self.metrics.mutes += 1;
        self.actions.push_back(ServerAction::SaveProfile(profile));
        Ok(())
    }

    /// Ends `req.peer_key`'s mute at `now`, as `moderator`. `existing` is
    /// the target's stored profile, as for [`Server::mute`]; without one
    /// there is nothing to persist. Queues the profile to persist.
    pub fn unmute(
        &mut self,
        now: Time,
        moderator: &Caller,
        req: &UnmuteRequest,
        existing: Option<Profile>,
    ) -> Result<(), ChatError> {
        if let Err(e) = self.check_moderation(moderator, &req.peer_key) {
            self.metrics.refused_moderation += 1;
            return Err(e);
        }
        self.remove_mute(&req.peer_key);
        self.metrics.unmutes += 1;
        if let Some(mut profile) = existing {
            profile.mute_deadline = now.unix();
            self.actions.push_back(ServerAction::SaveProfile(profile));
        }
        Ok(())
    }

    /// The caller's own mute: its deadline in unix seconds and its
    /// message, or 0 and no message when it isn't muted.
    pub fn get_mute(&self, now: Time, caller: &Caller) -> GetMuteResponse {
        match self.muted(now, &caller.peer_key) {
            Some(m) => GetMuteResponse {
                end_time: m.deadline,
                message: m.message.clone(),
            },
            None => GetMuteResponse::default(),
        }
    }

    /// Replaces the active mutes with those of the stored `profiles`. Of
    /// more than [`ServerLimits::max_mutes`], those that end last are
    /// kept. A mute's message is that of the profile's latest mute when
    /// that mute set the deadline.
    pub fn set_mutes<'a>(&mut self, now: Time, profiles: impl IntoIterator<Item = &'a Profile>) {
        self.mutes.clear();
        self.mutes_by_deadline.clear();
        let max = self.limits.max_mutes.max(1);
        for p in profiles {
            if !active(p.mute_deadline, now) || p.peer_key.len() > MAX_PEER_KEY_BYTES {
                continue;
            }
            self.remove_mute(&p.peer_key);
            if self.mutes.len() >= max {
                self.metrics.mutes_dropped += 1;
                if self
                    .mutes_by_deadline
                    .first()
                    .is_none_or(|(d, _)| *d >= p.mute_deadline)
                {
                    continue;
                }
                if let Some((_, k)) = self.mutes_by_deadline.pop_first() {
                    self.mutes.remove(&k);
                }
            }
            let message = p
                .mutes
                .last()
                .filter(|m| {
                    m.created_at
                        .checked_add(i64::from(m.duration_secs))
                        .is_some_and(|end| end == p.mute_deadline)
                })
                .map(|m| truncate(&m.message, MAX_MUTE_MESSAGE_BYTES).to_owned())
                .unwrap_or_default();
            self.insert_mute(
                p.peer_key.clone(),
                Mute {
                    deadline: p.mute_deadline,
                    message,
                },
            );
        }
    }

    /// The active mutes held.
    pub fn mute_count(&self) -> usize {
        self.mutes.len()
    }

    fn muted(&self, now: Time, peer_key: &[u8]) -> Option<&Mute> {
        self.mutes.get(peer_key).filter(|m| active(m.deadline, now))
    }

    fn insert_mute(&mut self, peer_key: Vec<u8>, mute: Mute) {
        self.mutes_by_deadline
            .insert((mute.deadline, peer_key.clone()));
        self.mutes.insert(peer_key, mute);
    }

    fn remove_mute(&mut self, peer_key: &[u8]) {
        if let Some(m) = self.mutes.remove(peer_key) {
            self.mutes_by_deadline
                .remove(&(m.deadline, peer_key.to_vec()));
        }
    }

    /// Forgets the mutes that ended by `now`.
    fn prune_mutes(&mut self, now: Time) {
        while let Some((d, _)) = self.mutes_by_deadline.first() {
            if active(*d, now) {
                break;
            }
            if let Some((_, k)) = self.mutes_by_deadline.pop_first() {
                self.mutes.remove(&k);
            }
        }
    }

    /// Replaces the nicks with the listing's `users`.
    pub fn reset_nicks(&mut self, users: &[User]) {
        self.ctx.nicks_mut().clear();
        self.aliases.clear();
        for u in users {
            self.add_nick(u);
        }
    }

    /// Follows a user event of the server's listing.
    pub fn nick_changed(&mut self, kind: UserEventKind, user: &User) {
        match kind {
            UserEventKind::Joined | UserEventKind::Renamed => self.add_nick(user),
            UserEventKind::Parted => self.remove_nick(&user.peer_key),
        }
    }

    fn add_nick(&mut self, user: &User) {
        if self
            .aliases
            .get(&user.peer_key)
            .is_some_and(|a| *a != user.alias)
        {
            self.remove_nick(&user.peer_key);
        }
        if !self.aliases.contains_key(&user.peer_key) && self.aliases.len() >= MAX_NICKS {
            self.metrics.nicks_dropped += 1;
            return;
        }
        if self
            .ctx
            .nicks_mut()
            .insert(&user.alias, user.peer_key.clone())
        {
            self.aliases
                .insert(user.peer_key.clone(), user.alias.clone());
        } else {
            self.metrics.nicks_dropped += 1;
        }
    }

    /// Removes `peer_key`'s nick, unless another user's nick replaced it.
    fn remove_nick(&mut self, peer_key: &[u8]) {
        let Some(alias) = self.aliases.remove(peer_key) else {
            return;
        };
        if self
            .ctx
            .nicks()
            .get(&alias)
            .is_some_and(|e| e.peer_key == peer_key)
        {
            self.ctx.nicks_mut().remove(&alias);
        }
    }

    /// The next action for the control.
    pub fn poll_action(&mut self) -> Option<ServerAction> {
        self.actions.pop_front()
    }

    pub fn metrics(&self) -> &ServerMetrics {
        &self.metrics
    }
}

/// Whether a deadline of `deadline` unix seconds is after `now`.
fn active(deadline: i64, now: Time) -> bool {
    Time::from_unix(deadline, 0).after(now)
}

fn check_caller(caller: &Caller) -> Result<(), ChatError> {
    if caller.peer_key.len() > MAX_PEER_KEY_BYTES || caller.alias.len() > MAX_ALIAS_BYTES {
        return Err(ChatError::TooLarge);
    }
    Ok(())
}

/// The longest prefix of `s` of at most `max` bytes that ends on a char
/// boundary.
fn truncate(s: &str, max: usize) -> &str {
    let mut end = max.min(s.len());
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s.get(..end).unwrap_or_default()
}

#[cfg(test)]
#[path = "server_test.rs"]
mod tests;
