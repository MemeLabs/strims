// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

use std::rc::Rc;

use strims_dao::schema::profile::{Devices, Profile};
use strims_dao::{Error, Singleton, Table};
use strims_kv::Kv;
use strims_proto::notification::v1::register_notification_frontend_service;
use strims_proto::profile::v1::{
    DeleteDeviceRequest, DeleteDeviceResponse, GetDeviceRequest, GetDeviceResponse,
    GetProfileRequest, GetProfileResponse, ListDevicesRequest, ListDevicesResponse,
    ProfileFrontendService, register_profile_frontend_service,
};
use strims_proto::replication::v1::register_replication_frontend_service;
use strims_proto::service::{Context, Error as RpcError};
use strims_replication::ReplicationFrontend;
use strims_rpc::{Router, Timer};

use crate::manager::Session;
use crate::notification::NotificationService;
use crate::rpc_error;

/// Serves `strims.profile.v1.ProfileFrontend` for a session.
pub struct ProfileService<K: Kv, T: Timer> {
    session: Rc<Session<K>>,
    timer: T,
}

impl<K: Kv, T: Timer> ProfileService<K, T> {
    /// The service of `session`.
    pub fn new(session: Rc<Session<K>>, timer: T) -> Self {
        ProfileService { session, timer }
    }
}

impl<K: Kv, T: Timer> ProfileFrontendService for ProfileService<K, T> {
    async fn get(&self, _: &Context, _: GetProfileRequest) -> Result<GetProfileResponse, RpcError> {
        let mut tx = self.session.store.read().await.map_err(rpc_error)?;
        let profile = Profile::get(&mut tx).await.map_err(rpc_error)?;
        Ok(GetProfileResponse { profile })
    }

    async fn delete_device(
        &self,
        _: &Context,
        req: DeleteDeviceRequest,
    ) -> Result<DeleteDeviceResponse, RpcError> {
        if req.id == self.session.store.replica_id() {
            return Err(RpcError::new(0, "cannot delete the current device"));
        }
        let mut tx = self
            .session
            .store
            .write(self.timer.now())
            .await
            .map_err(rpc_error)?;
        if !Devices::delete(&mut tx, req.id).await.map_err(rpc_error)? {
            return Err(rpc_error(Error::NotFound));
        }
        tx.commit().await.map_err(rpc_error)?;
        Ok(DeleteDeviceResponse {})
    }

    async fn get_device(
        &self,
        _: &Context,
        req: GetDeviceRequest,
    ) -> Result<GetDeviceResponse, RpcError> {
        let mut tx = self.session.store.read().await.map_err(rpc_error)?;
        let device = Devices::get(&mut tx, req.id)
            .await
            .map_err(rpc_error)?
            .ok_or_else(|| rpc_error(Error::NotFound))?;
        Ok(GetDeviceResponse {
            device: Some(device),
        })
    }

    async fn list_devices(
        &self,
        _: &Context,
        _: ListDevicesRequest,
    ) -> Result<ListDevicesResponse, RpcError> {
        let current = self.session.store.replica_id();
        let mut tx = self.session.store.read().await.map_err(rpc_error)?;
        let mut res = ListDevicesResponse::default();
        for d in Devices::get_all(&mut tx).await.map_err(rpc_error)? {
            if d.id == current {
                res.current_device = Some(d);
            } else {
                res.devices.push(d);
            }
        }
        Ok(res)
    }
}

/// Serves a session's profile, replication and notification frontend
/// services on `router`.
pub fn bind_frontends<K: Kv, T: Timer>(session: &Rc<Session<K>>, timer: &T, router: &Router) {
    let mut router = router.clone();
    register_profile_frontend_service(
        &mut router,
        Rc::new(ProfileService::new(session.clone(), timer.clone())),
    );
    register_replication_frontend_service(
        &mut router,
        Rc::new(ReplicationFrontend::new(
            session.store.clone(),
            timer.clone(),
        )),
    );
    register_notification_frontend_service(
        &mut router,
        Rc::new(NotificationService::new(
            session.store.clone(),
            timer.clone(),
        )),
    );
}
