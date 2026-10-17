// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! `ChatServerFrontend`: the profile's chat servers and their assets.

use std::collections::BTreeSet;

use prost::Message;
use strims_crypto::generate_key;
use strims_dao::schema::chat::{ChatEmotes, ChatModifiers, ChatServerIcons, ChatServers, ChatTags};
use strims_dao::{Index, Table, WriteTx, ref_key};
use strims_directory::Host;
use strims_kv::Kv;
use strims_proto::chat::v1::list_emotes_request::Part;
use strims_proto::chat::v1::{
    ChatServerFrontendService, CreateEmoteRequest, CreateEmoteResponse, CreateModifierRequest,
    CreateModifierResponse, CreateServerRequest, CreateServerResponse, CreateTagRequest,
    CreateTagResponse, DeleteEmoteRequest, DeleteEmoteResponse, DeleteModifierRequest,
    DeleteModifierResponse, DeleteServerRequest, DeleteServerResponse, DeleteTagRequest,
    DeleteTagResponse, Emote, GetEmoteRequest, GetEmoteResponse, GetModifierRequest,
    GetModifierResponse, GetServerIconRequest, GetServerIconResponse, GetServerRequest,
    GetServerResponse, GetTagRequest, GetTagResponse, ListEmoteLabelsRequest,
    ListEmoteLabelsResponse, ListEmotesRequest, ListEmotesResponse, ListModifiersRequest,
    ListModifiersResponse, ListServersRequest, ListServersResponse, ListTagsRequest,
    ListTagsResponse, Modifier, Room, Server, ServerIcon, SyncAssetsRequest, SyncAssetsResponse,
    Tag, UpdateEmoteRequest, UpdateEmoteResponse, UpdateEmotesRequest, UpdateEmotesResponse,
    UpdateModifierRequest, UpdateModifierResponse, UpdateServerIconRequest,
    UpdateServerIconResponse, UpdateServerRequest, UpdateServerResponse, UpdateTagRequest,
    UpdateTagResponse,
};
use strims_proto::service::{Context, Error as RpcError};
use strims_rpc::Timer;

use super::{ChatFrontend, MAX_LABELS, MAX_NAME_BYTES, MAX_RECORD_BYTES, not_found, rpc_error};
use crate::ChatFailure;
use crate::reader::RoomLimits;

fn check_name(what: &str, name: &str) -> Result<(), RpcError> {
    if name.is_empty() {
        return Err(rpc_error(format!("{what} name is empty")));
    }
    if name.len() > MAX_NAME_BYTES {
        return Err(rpc_error(format!("{what} name too long")));
    }
    Ok(())
}

fn check_size(what: &str, r: &impl Message) -> Result<(), RpcError> {
    if r.encoded_len() > MAX_RECORD_BYTES {
        return Err(rpc_error(format!("{what} too large")));
    }
    Ok(())
}

fn check_room(room: Option<&Room>) -> Result<(), RpcError> {
    if room.is_some_and(|r| r.name.len() > MAX_NAME_BYTES) {
        return Err(rpc_error("room name too long"));
    }
    Ok(())
}

fn check_labels(labels: &[String]) -> Result<(), RpcError> {
    if labels.len() > MAX_LABELS {
        return Err(rpc_error("too many labels"));
    }
    labels.iter().try_for_each(|l| check_name("label", l))
}

fn check_emote(e: &Emote) -> Result<(), RpcError> {
    check_name("emote", &e.name)?;
    check_labels(&e.labels)?;
    check_size("emote", e)
}

fn check_modifier(m: &Modifier) -> Result<(), RpcError> {
    check_name("modifier", &m.name)?;
    if !(0.0..=1.0).contains(&m.proc_chance) {
        return Err(rpc_error("proc chance must be from 0 to 1"));
    }
    check_size("modifier", m)
}

fn check_tag(t: &Tag) -> Result<(), RpcError> {
    check_name("tag", &t.name)?;
    if t.color.len() > MAX_NAME_BYTES {
        return Err(rpc_error("tag color too long"));
    }
    check_size("tag", t)
}

/// What one emote shows of the parts a `ListEmotes` call asks for: META,
/// ASSETS, or every field when it asks for none.
#[derive(Clone, Copy)]
struct Parts {
    meta: bool,
    assets: bool,
}

impl Parts {
    fn new(parts: &[i32]) -> Result<Parts, RpcError> {
        if parts.is_empty() {
            return Ok(Parts {
                meta: true,
                assets: true,
            });
        }
        let mut p = Parts {
            meta: false,
            assets: false,
        };
        for part in parts {
            match Part::try_from(*part) {
                Ok(Part::Meta) => p.meta = true,
                Ok(Part::Assets) => p.assets = true,
                _ => return Err(rpc_error("invalid part")),
            }
        }
        Ok(p)
    }

    fn apply(self, e: Emote) -> Emote {
        let Emote {
            id,
            server_id,
            name,
            images,
            effects,
            contributor,
            labels,
            enable,
        } = e;
        let (name, contributor, labels, enable) = if self.meta {
            (name, contributor, labels, enable)
        } else {
            Default::default()
        };
        let (images, effects) = if self.assets {
            (images, effects)
        } else {
            Default::default()
        };
        Emote {
            id,
            server_id,
            name,
            images,
            effects,
            contributor,
            labels,
            enable,
        }
    }
}

impl<K: Kv, H: Host, T: Timer> ChatFrontend<K, H, T> {
    fn limits(&self) -> RoomLimits {
        self.chat.shared.config.room
    }

    fn check_network(&self, key: &[u8]) -> Result<(), RpcError> {
        match self.chat.network().network(key) {
            Some(_) => Ok(()),
            None => Err(not_found("network")),
        }
    }

    /// Checks, in `tx`, that the profile has server `server_id` and that
    /// it has fewer than `max` records of `R` in `ix`.
    async fn check_room_for<R: Table>(
        tx: &mut WriteTx<K>,
        server_id: u64,
        ix: &Index<R::Record>,
        max: usize,
        what: &str,
    ) -> Result<(), RpcError> {
        if ChatServers::get(tx, server_id)
            .await
            .map_err(rpc_error)?
            .is_none()
        {
            return Err(not_found("chat server"));
        }
        let n = R::index_ids(tx, ix, &ref_key(server_id))
            .await
            .map_err(rpc_error)?
            .len();
        if n >= max {
            return Err(rpc_error(format!("too many {what}")));
        }
        Ok(())
    }

    /// Stores the record `make` makes with a new id as one of server
    /// `server_id`'s records of `R`, of which `ix` indexes fewer than `max`.
    async fn create<R: Table>(
        &self,
        server_id: u64,
        ix: &Index<R::Record>,
        max: usize,
        what: &str,
        make: impl FnOnce(u64) -> R::Record,
    ) -> Result<R::Record, RpcError> {
        let mut tx = self.write().await?;
        Self::check_room_for::<R>(&mut tx, server_id, ix, max, what).await?;
        let id = tx.generate_id().await.map_err(rpc_error)?;
        let r = R::insert(&mut tx, make(id)).await.map_err(rpc_error)?;
        tx.commit().await.map_err(rpc_error)?;
        Ok(r)
    }

    /// Stores the record `f` makes of record `id` of `R`.
    async fn change<R: Table>(
        &self,
        id: u64,
        what: &str,
        f: impl FnOnce(&mut R::Record) -> Result<(), RpcError>,
    ) -> Result<R::Record, RpcError> {
        let mut tx = self.write().await?;
        let mut r = R::get(&mut tx, id)
            .await
            .map_err(rpc_error)?
            .ok_or_else(|| not_found(what))?;
        f(&mut r)?;
        let r = R::update(&mut tx, r).await.map_err(rpc_error)?;
        tx.commit().await.map_err(rpc_error)?;
        Ok(r)
    }

    async fn delete<R: Table>(&self, id: u64, what: &str) -> Result<(), RpcError> {
        let mut tx = self.write().await?;
        if !R::delete(&mut tx, id).await.map_err(rpc_error)? {
            return Err(not_found(what));
        }
        tx.commit().await.map_err(rpc_error)
    }

    async fn get<R: Table>(&self, id: u64, what: &str) -> Result<R::Record, RpcError> {
        let mut tx = self.store().read().await.map_err(rpc_error)?;
        R::get(&mut tx, id)
            .await
            .map_err(rpc_error)?
            .ok_or_else(|| not_found(what))
    }

    /// Server `server_id`'s records of `R`, which `ix` indexes.
    async fn list<R: Table>(
        &self,
        ix: &Index<R::Record>,
        server_id: u64,
    ) -> Result<Vec<R::Record>, RpcError> {
        let mut tx = self.store().read().await.map_err(rpc_error)?;
        R::index_get_all(&mut tx, ix, &ref_key(server_id))
            .await
            .map_err(rpc_error)
    }
}

impl<K: Kv, H: Host, T: Timer> ChatServerFrontendService for ChatFrontend<K, H, T> {
    /// A server on one of the profile's networks, with a new key.
    async fn create_server(
        &self,
        _: &Context,
        req: CreateServerRequest,
    ) -> Result<CreateServerResponse, RpcError> {
        self.check_network(&req.network_key)?;
        check_room(req.room.as_ref())?;
        let key = generate_key(&mut self.chat.shared.rng.borrow_mut());
        let mut tx = self.write().await?;
        let server = Server {
            id: tx.generate_id().await.map_err(rpc_error)?,
            network_key: req.network_key,
            key: Some(key),
            room: req.room,
            admin_peer_keys: Vec::new(),
        };
        check_size("chat server", &server)?;
        let server = ChatServers::insert(&mut tx, server)
            .await
            .map_err(rpc_error)?;
        tx.commit().await.map_err(rpc_error)?;
        Ok(CreateServerResponse {
            server: Some(server),
        })
    }

    /// Changes a server's network and room. A server moved to another
    /// network serves there.
    async fn update_server(
        &self,
        _: &Context,
        req: UpdateServerRequest,
    ) -> Result<UpdateServerResponse, RpcError> {
        self.check_network(&req.network_key)?;
        check_room(req.room.as_ref())?;
        let server = self
            .change::<ChatServers>(req.id, "chat server", |s| {
                s.network_key = req.network_key;
                s.room = req.room;
                check_size("chat server", s)
            })
            .await?;
        Ok(UpdateServerResponse {
            server: Some(server),
        })
    }

    /// Deletes a server with its icon, emotes, modifiers, tags and user
    /// profiles.
    async fn delete_server(
        &self,
        _: &Context,
        req: DeleteServerRequest,
    ) -> Result<DeleteServerResponse, RpcError> {
        self.delete::<ChatServers>(req.id, "chat server").await?;
        Ok(DeleteServerResponse {})
    }

    async fn get_server(
        &self,
        _: &Context,
        req: GetServerRequest,
    ) -> Result<GetServerResponse, RpcError> {
        let server = self.get::<ChatServers>(req.id, "chat server").await?;
        Ok(GetServerResponse {
            server: Some(server),
        })
    }

    async fn list_servers(
        &self,
        _: &Context,
        _: ListServersRequest,
    ) -> Result<ListServersResponse, RpcError> {
        let mut tx = self.store().read().await.map_err(rpc_error)?;
        let servers = ChatServers::get_all(&mut tx).await.map_err(rpc_error)?;
        Ok(ListServersResponse { servers })
    }

    /// Sets an existing server's icon.
    async fn update_server_icon(
        &self,
        _: &Context,
        req: UpdateServerIconRequest,
    ) -> Result<UpdateServerIconResponse, RpcError> {
        let icon = ServerIcon {
            id: req.server_id,
            image: req.image,
        };
        check_size("icon", &icon)?;
        let mut tx = self.write().await?;
        if ChatServers::get(&mut tx, req.server_id)
            .await
            .map_err(rpc_error)?
            .is_none()
        {
            return Err(not_found("chat server"));
        }
        let icon = ChatServerIcons::upsert(&mut tx, icon)
            .await
            .map_err(rpc_error)?;
        tx.commit().await.map_err(rpc_error)?;
        Ok(UpdateServerIconResponse {
            server_icon: Some(icon),
        })
    }

    /// A server's icon, none when it has none.
    async fn get_server_icon(
        &self,
        _: &Context,
        req: GetServerIconRequest,
    ) -> Result<GetServerIconResponse, RpcError> {
        let mut tx = self.store().read().await.map_err(rpc_error)?;
        let server_icon = ChatServerIcons::get(&mut tx, req.server_id)
            .await
            .map_err(rpc_error)?;
        Ok(GetServerIconResponse { server_icon })
    }

    /// An emote of an existing server with fewer emotes than its readers
    /// take ([`RoomLimits::max_emotes`]).
    async fn create_emote(
        &self,
        _: &Context,
        req: CreateEmoteRequest,
    ) -> Result<CreateEmoteResponse, RpcError> {
        let emote = Emote {
            id: 0,
            server_id: req.server_id,
            name: req.name,
            images: req.images,
            effects: req.effects,
            contributor: req.contributor,
            labels: req.labels,
            enable: req.enable,
        };
        check_emote(&emote)?;
        let max = self.limits().max_emotes;
        let emote = self
            .create::<ChatEmotes>(req.server_id, &ChatEmotes::BY_SERVER, max, "emotes", |id| {
                Emote { id, ..emote }
            })
            .await?;
        Ok(CreateEmoteResponse { emote: Some(emote) })
    }

    async fn update_emote(
        &self,
        _: &Context,
        req: UpdateEmoteRequest,
    ) -> Result<UpdateEmoteResponse, RpcError> {
        let emote = self
            .change::<ChatEmotes>(req.id, "emote", |e| {
                e.name = req.name;
                e.images = req.images;
                e.effects = req.effects;
                e.contributor = req.contributor;
                e.labels = req.labels;
                e.enable = req.enable;
                check_emote(e)
            })
            .await?;
        Ok(UpdateEmoteResponse { emote: Some(emote) })
    }

    async fn delete_emote(
        &self,
        _: &Context,
        req: DeleteEmoteRequest,
    ) -> Result<DeleteEmoteResponse, RpcError> {
        self.delete::<ChatEmotes>(req.id, "emote").await?;
        Ok(DeleteEmoteResponse {})
    }

    async fn get_emote(
        &self,
        _: &Context,
        req: GetEmoteRequest,
    ) -> Result<GetEmoteResponse, RpcError> {
        let emote = self.get::<ChatEmotes>(req.id, "emote").await?;
        Ok(GetEmoteResponse { emote: Some(emote) })
    }

    /// A server's emotes, with the fields of the parts asked for: META is
    /// the name, contributor, labels and whether it's enabled, and ASSETS
    /// the images and effects. Without parts, every field.
    async fn list_emotes(
        &self,
        _: &Context,
        req: ListEmotesRequest,
    ) -> Result<ListEmotesResponse, RpcError> {
        let parts = Parts::new(&req.parts)?;
        let emotes = self
            .list::<ChatEmotes>(&ChatEmotes::BY_SERVER, req.server_id)
            .await?
            .into_iter()
            .map(|e| parts.apply(e))
            .collect();
        Ok(ListEmotesResponse { emotes })
    }

    /// Adds and removes labels on several emotes, and enables or disables
    /// them, in one write. Each emote's labels end up sorted and unique.
    async fn update_emotes(
        &self,
        _: &Context,
        req: UpdateEmotesRequest,
    ) -> Result<UpdateEmotesResponse, RpcError> {
        if req.ids.len() > self.limits().max_emotes {
            return Err(rpc_error("too many emotes"));
        }
        check_labels(&req.add_labels)?;
        check_labels(&req.remove_labels)?;
        let mut tx = self.write().await?;
        for id in req.ids {
            let mut e = ChatEmotes::get(&mut tx, id)
                .await
                .map_err(rpc_error)?
                .ok_or_else(|| not_found("emote"))?;
            let mut labels: BTreeSet<String> = std::mem::take(&mut e.labels).into_iter().collect();
            labels.extend(req.add_labels.iter().cloned());
            for l in &req.remove_labels {
                labels.remove(l);
            }
            e.labels = labels.into_iter().collect();
            if let Some(enable) = req.enable {
                e.enable = enable;
            }
            check_emote(&e)?;
            ChatEmotes::update(&mut tx, e).await.map_err(rpc_error)?;
        }
        tx.commit().await.map_err(rpc_error)?;
        Ok(UpdateEmotesResponse {})
    }

    /// The labels of a server's emotes, sorted.
    async fn list_emote_labels(
        &self,
        _: &Context,
        req: ListEmoteLabelsRequest,
    ) -> Result<ListEmoteLabelsResponse, RpcError> {
        let labels: BTreeSet<String> = self
            .list::<ChatEmotes>(&ChatEmotes::BY_SERVER, req.server_id)
            .await?
            .into_iter()
            .flat_map(|e| e.labels)
            .collect();
        Ok(ListEmoteLabelsResponse {
            labels: labels.into_iter().collect(),
        })
    }

    /// A modifier of an existing server with fewer modifiers than its
    /// readers take ([`RoomLimits::max_modifiers`]). Its proc chance is
    /// from 0 to 1.
    async fn create_modifier(
        &self,
        _: &Context,
        req: CreateModifierRequest,
    ) -> Result<CreateModifierResponse, RpcError> {
        let modifier = Modifier {
            id: 0,
            server_id: req.server_id,
            name: req.name,
            priority: req.priority,
            internal: req.internal,
            extra_wrap_count: req.extra_wrap_count,
            proc_chance: req.proc_chance,
            style_sheet: req.style_sheet,
        };
        check_modifier(&modifier)?;
        let max = self.limits().max_modifiers;
        let modifier = self
            .create::<ChatModifiers>(
                req.server_id,
                &ChatModifiers::BY_SERVER,
                max,
                "modifiers",
                |id| Modifier { id, ..modifier },
            )
            .await?;
        Ok(CreateModifierResponse {
            modifier: Some(modifier),
        })
    }

    async fn update_modifier(
        &self,
        _: &Context,
        req: UpdateModifierRequest,
    ) -> Result<UpdateModifierResponse, RpcError> {
        let modifier = self
            .change::<ChatModifiers>(req.id, "modifier", |m| {
                m.name = req.name;
                m.priority = req.priority;
                m.internal = req.internal;
                m.extra_wrap_count = req.extra_wrap_count;
                m.proc_chance = req.proc_chance;
                m.style_sheet = req.style_sheet;
                check_modifier(m)
            })
            .await?;
        Ok(UpdateModifierResponse {
            modifier: Some(modifier),
        })
    }

    async fn delete_modifier(
        &self,
        _: &Context,
        req: DeleteModifierRequest,
    ) -> Result<DeleteModifierResponse, RpcError> {
        self.delete::<ChatModifiers>(req.id, "modifier").await?;
        Ok(DeleteModifierResponse {})
    }

    async fn get_modifier(
        &self,
        _: &Context,
        req: GetModifierRequest,
    ) -> Result<GetModifierResponse, RpcError> {
        let modifier = self.get::<ChatModifiers>(req.id, "modifier").await?;
        Ok(GetModifierResponse {
            modifier: Some(modifier),
        })
    }

    async fn list_modifiers(
        &self,
        _: &Context,
        req: ListModifiersRequest,
    ) -> Result<ListModifiersResponse, RpcError> {
        let modifiers = self
            .list::<ChatModifiers>(&ChatModifiers::BY_SERVER, req.server_id)
            .await?;
        Ok(ListModifiersResponse { modifiers })
    }

    /// A tag of an existing server with fewer tags than its readers take
    /// ([`RoomLimits::max_tags`]).
    async fn create_tag(
        &self,
        _: &Context,
        req: CreateTagRequest,
    ) -> Result<CreateTagResponse, RpcError> {
        let tag = Tag {
            id: 0,
            server_id: req.server_id,
            name: req.name,
            color: req.color,
            sensitive: req.sensitive,
        };
        check_tag(&tag)?;
        let max = self.limits().max_tags;
        let tag = self
            .create::<ChatTags>(req.server_id, &ChatTags::BY_SERVER, max, "tags", |id| Tag {
                id,
                ..tag
            })
            .await?;
        Ok(CreateTagResponse { tag: Some(tag) })
    }

    async fn update_tag(
        &self,
        _: &Context,
        req: UpdateTagRequest,
    ) -> Result<UpdateTagResponse, RpcError> {
        let tag = self
            .change::<ChatTags>(req.id, "tag", |t| {
                t.name = req.name;
                t.color = req.color;
                t.sensitive = req.sensitive;
                check_tag(t)
            })
            .await?;
        Ok(UpdateTagResponse { tag: Some(tag) })
    }

    async fn delete_tag(
        &self,
        _: &Context,
        req: DeleteTagRequest,
    ) -> Result<DeleteTagResponse, RpcError> {
        self.delete::<ChatTags>(req.id, "tag").await?;
        Ok(DeleteTagResponse {})
    }

    async fn get_tag(&self, _: &Context, req: GetTagRequest) -> Result<GetTagResponse, RpcError> {
        let tag = self.get::<ChatTags>(req.id, "tag").await?;
        Ok(GetTagResponse { tag: Some(tag) })
    }

    async fn list_tags(
        &self,
        _: &Context,
        req: ListTagsRequest,
    ) -> Result<ListTagsResponse, RpcError> {
        let tags = self
            .list::<ChatTags>(&ChatTags::BY_SERVER, req.server_id)
            .await?;
        Ok(ListTagsResponse { tags })
    }

    /// Has the server publish its assets: what changed since its last
    /// bundle, or with `force_unified_update` a unified bundle. A server of
    /// the profile this process doesn't run isn't refused: its assets go
    /// out in the unified bundle it starts serving with.
    async fn sync_assets(
        &self,
        _: &Context,
        req: SyncAssetsRequest,
    ) -> Result<SyncAssetsResponse, RpcError> {
        match self
            .chat
            .sync_assets(req.server_id, req.force_unified_update)
        {
            Ok(()) => {}
            Err(ChatFailure::NoServer) => {
                self.get::<ChatServers>(req.server_id, "chat server")
                    .await?;
            }
            Err(e) => return Err(e.to_rpc()),
        }
        Ok(SyncAssetsResponse::default())
    }
}
