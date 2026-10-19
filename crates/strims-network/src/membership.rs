// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The records a profile's memberships start from: a network it creates
//! and runs the CA of, an invitation to one, and a network joined from an
//! invitation.

use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};
use strims_crypto::{
    DEFAULT_CERT_TTL, Error as CryptoError, Issuer, generate_key, new_certificate_request,
    sign_certificate_request, sign_certificate_request_with_network, verify_certificate,
};
use strims_proto::network::v1::directory::ServerConfig as DirectoryConfig;
use strims_proto::network::v1::directory::server_config::Integrations;
use strims_proto::network::v1::directory::server_config::integrations::Swarm;
use strims_proto::network::v1::{InvitationV0, Network, ServerConfig};
use strims_proto::profile::v1::Profile;
use strims_proto::types::{Certificate, Image, KeyUsage};

use crate::ca::MEMBER_USAGE;

/// How long an invitation lasts.
pub const INVITATION_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Why a membership record couldn't be made.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MembershipError {
    #[error("profile has no key")]
    NoProfileKey,
    #[error("invitation incomplete")]
    InvalidInvitation,
    #[error(transparent)]
    Crypto(#[from] CryptoError),
    #[error("invalid certificate: {0}")]
    Certificate(String),
}

fn subject<'a>(profile: &'a Profile, alias: &'a str) -> &'a str {
    if alias.is_empty() {
        &profile.name
    } else {
        alias
    }
}

/// A network `profile` creates and runs: a new root key in its server
/// config, and the profile's membership under it, with `alias` (or the
/// profile's name) as its subject.
pub fn new_network(
    id: u64,
    name: &str,
    icon: Option<Image>,
    profile: &Profile,
    alias: &str,
    now: Time,
    rng: &mut DetRng,
) -> Result<Network, MembershipError> {
    let profile_key = profile.key.as_ref().ok_or(MembershipError::NoProfileKey)?;
    let key = generate_key(rng);
    let csr = new_certificate_request(profile_key, MEMBER_USAGE, subject(profile, alias))?;
    let certificate =
        sign_certificate_request_with_network(&csr, &key, name, DEFAULT_CERT_TTL, now, rng)?;
    let alias = if csr.subject == profile.name {
        String::new()
    } else {
        csr.subject.clone()
    };
    Ok(Network {
        id,
        certificate: Some(certificate),
        alias,
        server_config: Some(ServerConfig {
            name: name.to_owned(),
            key: Some(key),
            directory: Some(DirectoryConfig {
                integrations: Some(Integrations {
                    swarm: Some(Swarm { enable: true }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            icon,
            ..Default::default()
        }),
        ..Default::default()
    })
}

/// An invitation to the network of `membership`, the profile's
/// certificate on it: a new key, with a certificate the profile signs
/// that lasts [`INVITATION_TTL`].
pub fn new_invitation_v0(
    profile: &Profile,
    membership: &Certificate,
    now: Time,
    rng: &mut DetRng,
) -> Result<InvitationV0, MembershipError> {
    let profile_key = profile.key.as_ref().ok_or(MembershipError::NoProfileKey)?;
    let key = generate_key(rng);
    let csr = new_certificate_request(&key, KeyUsage::Sign as i32, "")?;
    let issuer = Issuer::Certificate {
        key: profile_key,
        certificate: membership,
    };
    let certificate = sign_certificate_request(&csr, INVITATION_TTL, issuer, now, rng)?;
    Ok(InvitationV0 {
        key: Some(key),
        certificate: Some(certificate),
        ..Default::default()
    })
}

/// The network `profile` joins from `invitation`: a membership signed by
/// the invitation's key, under its certificate, which the profile renews
/// at the network's CA before peers bind it.
pub fn network_from_invitation_v0(
    id: u64,
    invitation: &InvitationV0,
    profile: &Profile,
    alias: &str,
    now: Time,
    rng: &mut DetRng,
) -> Result<Network, MembershipError> {
    let profile_key = profile.key.as_ref().ok_or(MembershipError::NoProfileKey)?;
    let (Some(key), Some(invite)) = (&invitation.key, &invitation.certificate) else {
        return Err(MembershipError::InvalidInvitation);
    };
    verify_certificate(invite, now).map_err(|e| MembershipError::Certificate(e.to_string()))?;
    let csr = new_certificate_request(profile_key, KeyUsage::Peer as i32, subject(profile, alias))?;
    let issuer = Issuer::Certificate {
        key,
        certificate: invite,
    };
    let certificate = sign_certificate_request(&csr, DEFAULT_CERT_TTL, issuer, now, rng)?;
    Ok(Network {
        id,
        certificate: Some(certificate),
        alias: alias.to_owned(),
        ..Default::default()
    })
}
