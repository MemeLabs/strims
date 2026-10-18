// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

use std::rc::Rc;

use strims_dao::auth::{self, ProfileKey, SessionToken};
use strims_dao::seal::StorageKey;
use strims_kv::Kv;
use strims_proto::auth::v1::{
    AuthFrontendService, LinkedProfile, SignInRequest, SignInResponse, SignUpRequest,
    SignUpResponse, linked_profile, sign_in_request,
};
use strims_proto::service::{Context, Error as RpcError};
use strims_rpc::{Router, Timer, WeakRouter};

use crate::manager::{Manager, SessionFactory};
use crate::rpc_error;

/// Serves `strims.auth.v1.AuthFrontend` on one frontend connection, and
/// binds the signed-in profile's services on the connection's router.
pub struct AuthService<K: Kv + Clone, F: SessionFactory<K>, T: Timer> {
    manager: Rc<Manager<K, F, T>>,
    session_key: Option<StorageKey>,
    /// Weak: the router holds this service.
    router: WeakRouter,
}

impl<K: Kv + Clone, F: SessionFactory<K>, T: Timer> AuthService<K, F, T> {
    /// The service for a connection served by `router`. A connection with a
    /// `session_key` persists sign-ins as session tokens sealed under it;
    /// one without gets the profile key itself.
    pub fn new(
        manager: Rc<Manager<K, F, T>>,
        session_key: Option<StorageKey>,
        router: Router,
    ) -> Self {
        AuthService {
            manager,
            session_key,
            router: router.downgrade(),
        }
    }

    /// What a persisted sign-in gives the client to sign in with again.
    async fn credentials(
        &self,
        pk: &ProfileKey,
    ) -> Result<linked_profile::Credentials, strims_dao::Error> {
        let Some(sk) = &self.session_key else {
            return Ok(linked_profile::Credentials::Key(linked_profile::Key {
                profile_id: pk.id,
                profile_key: pk.key.to_vec(),
            }));
        };
        let mut rng = self.manager.fork_rng();
        let t = auth::create_session(
            self.manager.kv(),
            sk,
            pk,
            &mut rng,
            self.manager.timer().now(),
        )
        .await?;
        Ok(linked_profile::Credentials::Token(linked_profile::Token {
            profile_id: pk.id,
            token: t.token,
            eol: t.eol,
        }))
    }

    /// Starts or joins `pk`'s session and binds its services here.
    async fn bind(
        &self,
        pk: &ProfileKey,
        credentials: Option<linked_profile::Credentials>,
    ) -> Result<(LinkedProfile, strims_proto::profile::v1::Profile), RpcError> {
        let (session, runtime) = self.manager.get_or_create(pk).await.map_err(rpc_error)?;
        let router = self
            .router
            .upgrade()
            .ok_or_else(|| RpcError::new(0, "connection closed".to_owned()))?;
        self.manager.factory().bind(&runtime, &session, &router);
        let linked = LinkedProfile {
            name: session.profile.name.clone(),
            credentials,
            ..Default::default()
        };
        Ok((linked, session.profile.clone()))
    }
}

impl<K: Kv + Clone, F: SessionFactory<K>, T: Timer> AuthFrontendService for AuthService<K, F, T> {
    async fn sign_in(&self, _: &Context, req: SignInRequest) -> Result<SignInResponse, RpcError> {
        let kv = self.manager.kv();
        let now = self.manager.timer().now();
        let (pk, persist) = match req.credentials {
            Some(sign_in_request::Credentials::Password(p)) => {
                let pk = match &p.pairing_token {
                    Some(token) => {
                        let user = token
                            .auth
                            .as_ref()
                            .ok_or_else(|| RpcError::new(0, "invalid pairing token"))?;
                        let pk = auth::open_user(user, &p.password).map_err(rpc_error)?;
                        let mut rng = self.manager.fork_rng();
                        auth::import_pairing_token(kv, token, &pk, &mut rng, now)
                            .await
                            .map_err(rpc_error)?;
                        pk
                    }
                    None => auth::sign_in(kv, &p.name, &p.password)
                        .await
                        .map_err(rpc_error)?,
                };
                (pk, p.persist_login)
            }
            Some(sign_in_request::Credentials::Token(t)) => {
                let sk = self
                    .session_key
                    .as_ref()
                    .ok_or(strims_dao::Error::Credentials)
                    .map_err(rpc_error)?;
                let token = SessionToken {
                    eol: t.eol,
                    token: t.token,
                };
                let pk = auth::load_session(kv, sk, &token, now)
                    .await
                    .map_err(rpc_error)?;
                (pk, false)
            }
            Some(sign_in_request::Credentials::Key(k)) => {
                let pk = ProfileKey::from_parts(k.profile_id, &k.profile_key).map_err(rpc_error)?;
                (pk, false)
            }
            None => return Err(RpcError::new(0, "missing credentials")),
        };
        let credentials = match persist {
            true => Some(self.credentials(&pk).await.map_err(rpc_error)?),
            false => None,
        };
        let (linked, profile) = self.bind(&pk, credentials).await?;
        Ok(SignInResponse {
            linked_profile: Some(linked),
            profile: Some(profile),
        })
    }

    async fn sign_up(&self, _: &Context, req: SignUpRequest) -> Result<SignUpResponse, RpcError> {
        if req.name.trim().is_empty() {
            return Err(RpcError::new(0, "a name is required"));
        }
        let now = self.manager.timer().now();
        let mut rng = self.manager.fork_rng();
        let (pk, _) = auth::sign_up(
            self.manager.kv(),
            &req.name,
            &req.password,
            self.manager.os(),
            &mut rng,
            now,
        )
        .await
        .map_err(rpc_error)?;
        let credentials = match req.persist_login {
            true => Some(self.credentials(&pk).await.map_err(rpc_error)?),
            false => None,
        };
        let (linked, profile) = self.bind(&pk, credentials).await?;
        Ok(SignUpResponse {
            linked_profile: Some(linked),
            profile: Some(profile),
        })
    }
}
