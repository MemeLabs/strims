// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The calls to a chat server: those a served server takes, from its
//! `chat.v1.Chat` service or from this node, and those this node makes to
//! remote servers.
//!
//! # Moderation
//!
//! A mute or unmute is checked first
//! ([`Server::check_moderation`](crate::Server::check_moderation)), then
//! runs in one store transaction: it stores the profiles an earlier,
//! dropped call left queued, reads the target's profile, applies the
//! moderation to the server and stores the profile it queues. A store's
//! write transactions don't interleave, so the moderation calls of a
//! server read each other's profiles in order.

use std::rc::{Rc, Weak};

use strims_dao::Table;
use strims_dao::schema::chat::{ChatProfiles, profile_peer_key};
use strims_directory::{Caller, Host};
use strims_kv::Kv;
use strims_proto::chat::v1::{
    ChatClient, ChatService, GetMuteRequest, GetMuteResponse, MuteRequest, MuteResponse,
    SendMessageRequest, SendMessageResponse, UnmuteRequest, UnmuteResponse,
};
use strims_proto::service::{Context, Error as RpcError};
use strims_rpc::{RetryClient, Timer};

use crate::ChatError;
use crate::control::{ChatFailure, Runner, Shared};

/// A call to a chat server.
pub(crate) enum Request {
    SendMessage(String),
    Mute(MuteRequest),
    Unmute(UnmuteRequest),
    GetMute,
}

/// A call's answer.
pub(crate) enum Reply {
    Done,
    Mute(GetMuteResponse),
}

/// Makes `req` through `client`.
pub(crate) async fn call_remote<T: Timer>(
    client: &ChatClient<RetryClient<T>>,
    req: Request,
) -> Result<Reply, ChatFailure> {
    let f = ChatFailure::from_call;
    Ok(match req {
        Request::SendMessage(body) => {
            client
                .send_message(&SendMessageRequest { body })
                .await
                .map_err(f)?;
            Reply::Done
        }
        Request::Mute(m) => {
            client.mute(&m).await.map_err(f)?;
            Reply::Done
        }
        Request::Unmute(m) => {
            client.unmute(&m).await.map_err(f)?;
            Reply::Done
        }
        Request::GetMute => Reply::Mute(client.get_mute(&GetMuteRequest {}).await.map_err(f)?),
    })
}

/// Serves `req` from `caller` with the server `r` runs.
pub(crate) async fn serve<K: Kv, H: Host, T: Timer>(
    s: &Shared<K, H, T>,
    r: &Runner<H>,
    caller: &Caller,
    req: Request,
) -> Result<Reply, ChatFailure> {
    match req {
        Request::SendMessage(body) => refused_or_done(s.with_server(r, |srv, now| {
            srv.send_message(now, caller, &body, &mut s.patterns())
        })),
        Request::GetMute => s
            .with_server(r, |srv, now| srv.get_mute(now, caller))
            .map(Reply::Mute)
            .ok_or(ChatFailure::NoServer),
        Request::Mute(m) => moderate(s, r, caller, Moderation::Mute(m)).await,
        Request::Unmute(m) => moderate(s, r, caller, Moderation::Unmute(m)).await,
    }
}

enum Moderation {
    Mute(MuteRequest),
    Unmute(UnmuteRequest),
}

impl Moderation {
    fn peer_key(&self) -> &[u8] {
        match self {
            Moderation::Mute(m) => &m.peer_key,
            Moderation::Unmute(m) => &m.peer_key,
        }
    }
}

/// Applies a mute or unmute; see the [module docs](self).
async fn moderate<K: Kv, H: Host, T: Timer>(
    s: &Shared<K, H, T>,
    r: &Runner<H>,
    caller: &Caller,
    m: Moderation,
) -> Result<Reply, ChatFailure> {
    s.with_server(r, |srv, _| srv.check_moderation(caller, m.peer_key()))
        .ok_or(ChatFailure::NoServer)?
        .map_err(ChatFailure::Refused)?;
    let server_id = r
        .server
        .borrow()
        .as_ref()
        .map(|st| st.server_id)
        .ok_or(ChatFailure::NoServer)?;
    let key = profile_peer_key(server_id, m.peer_key());
    let res = async {
        let mut tx = s.store.write(s.timer.now()).await?;
        let queued = r
            .server
            .borrow_mut()
            .as_mut()
            .map(|st| std::mem::take(&mut st.profiles))
            .unwrap_or_default();
        for p in queued {
            ChatProfiles::upsert(&mut tx, p).await?;
        }
        let existing = ChatProfiles::index_get(&mut tx, &ChatProfiles::BY_PEER_KEY, &key).await?;
        let new_id = match (&m, &existing) {
            (Moderation::Mute(_), None) => tx.generate_id().await?,
            _ => 0,
        };
        let res = s.with_server(r, |srv, now| match &m {
            Moderation::Mute(req) => srv.mute(now, caller, req, existing, new_id),
            Moderation::Unmute(req) => srv.unmute(now, caller, req, existing),
        });
        let queued = r
            .server
            .borrow_mut()
            .as_mut()
            .map(|st| std::mem::take(&mut st.profiles))
            .unwrap_or_default();
        for p in queued {
            ChatProfiles::upsert(&mut tx, p).await?;
        }
        tx.commit().await?;
        Ok(res)
    }
    .await;
    match res {
        Ok(res) => refused_or_done(res),
        Err(e) => {
            s.metrics.borrow_mut().store_errors += 1;
            tracing::warn!(error = %e, "storing a chat profile failed");
            Err(ChatFailure::Store(e))
        }
    }
}

fn refused_or_done(res: Option<Result<(), ChatError>>) -> Result<Reply, ChatFailure> {
    res.ok_or(ChatFailure::NoServer)?
        .map(|()| Reply::Done)
        .map_err(ChatFailure::Refused)
}

/// `strims.chat.v1.Chat` for one server, on its port. The caller is the
/// vpn certificate's host.
pub(crate) struct ChatServer<K: Kv, H: Host, T: Timer> {
    shared: Weak<Shared<K, H, T>>,
    runner: Weak<Runner<H>>,
}

impl<K: Kv, H: Host, T: Timer> ChatServer<K, H, T> {
    pub(crate) fn new(s: &Rc<Shared<K, H, T>>, r: &Rc<Runner<H>>) -> Self {
        ChatServer {
            shared: Rc::downgrade(s),
            runner: Rc::downgrade(r),
        }
    }

    async fn serve(&self, ctx: &Context, req: Request) -> Result<Reply, RpcError> {
        let (Some(s), Some(r)) = (self.shared.upgrade(), self.runner.upgrade()) else {
            return Err(ChatFailure::NoServer.to_rpc());
        };
        let caller = ctx
            .certificate()
            .and_then(Caller::from_certificate)
            .ok_or_else(|| RpcError::new(0, "no caller certificate"))?;
        serve(&s, &r, &caller, req).await.map_err(|e| e.to_rpc())
    }
}

impl<K: Kv, H: Host, T: Timer> ChatService for ChatServer<K, H, T> {
    async fn send_message(
        &self,
        ctx: &Context,
        req: SendMessageRequest,
    ) -> Result<SendMessageResponse, RpcError> {
        self.serve(ctx, Request::SendMessage(req.body)).await?;
        Ok(SendMessageResponse {})
    }

    async fn mute(&self, ctx: &Context, req: MuteRequest) -> Result<MuteResponse, RpcError> {
        self.serve(ctx, Request::Mute(req)).await?;
        Ok(MuteResponse {})
    }

    async fn unmute(&self, ctx: &Context, req: UnmuteRequest) -> Result<UnmuteResponse, RpcError> {
        self.serve(ctx, Request::Unmute(req)).await?;
        Ok(UnmuteResponse {})
    }

    async fn get_mute(
        &self,
        ctx: &Context,
        _: GetMuteRequest,
    ) -> Result<GetMuteResponse, RpcError> {
        match self.serve(ctx, Request::GetMute).await? {
            Reply::Mute(m) => Ok(m),
            Reply::Done => Ok(GetMuteResponse::default()),
        }
    }
}
