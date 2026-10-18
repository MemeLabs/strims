// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The service's API over real WebSockets: its path, its session cookie,
//! and the frontends a sign-in binds.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use strims_proto::auth::v1::{
    AuthFrontendClient, SignInRequest, SignUpRequest, linked_profile, sign_in_request,
};
use strims_proto::network::v1::bootstrap::{BootstrapFrontendClient, GetConfigRequest};
use strims_proto::network::v1::{ListNetworksRequest, NetworkFrontendClient};
use support::*;

#[test]
fn the_api_answers_only_at_its_path() {
    run(async {
        let svc = service().await;
        assert!(connect_at(&svc, "/nope", None).await.is_err());
        let ui = connect_ui(&svc).await;
        let set = ui.set_cookie.as_deref().unwrap();
        assert!(set.starts_with("session_key="), "{set}");
        for attr in ["Max-Age=2592000", "HttpOnly", "Secure", "SameSite=None"] {
            assert!(set.contains(attr), "{set}");
        }

        // Before a sign-in, only the auth service answers.
        let networks = NetworkFrontendClient::new(ui.client.clone());
        assert!(networks.list(&ListNetworksRequest {}).await.is_err());
        AuthFrontendClient::new(ui.client.clone())
            .sign_up(&SignUpRequest {
                name: "alice".into(),
                password: "pw".into(),
                ..Default::default()
            })
            .await
            .unwrap();
        assert!(networks.list(&ListNetworksRequest {}).await.is_ok());
        let config = BootstrapFrontendClient::new(ui.client.clone())
            .get_config(&GetConfigRequest {})
            .await
            .unwrap();
        assert!(config.config.is_some());
        let info = svc.node_info().unwrap();
        assert!(info.ws_uri.unwrap().starts_with("ws://127.0.0.1:"));
        svc.stop().await;
    });
}

#[test]
fn session_cookies_persist_logins() {
    run(async {
        let svc = service().await;
        let ui = connect_ui(&svc).await;
        let cookie = ui.cookie();
        let res = AuthFrontendClient::new(ui.client.clone())
            .sign_up(&SignUpRequest {
                name: "alice".into(),
                password: "pw".into(),
                persist_session: false,
                persist_login: true,
            })
            .await
            .unwrap();
        let Some(linked_profile::Credentials::Token(token)) =
            res.linked_profile.unwrap().credentials
        else {
            panic!("a connection with a session key gets a token");
        };
        let sign_in = SignInRequest {
            credentials: Some(sign_in_request::Credentials::Token(
                sign_in_request::Token {
                    profile_id: token.profile_id,
                    token: token.token.clone(),
                    eol: token.eol,
                },
            )),
        };

        // The browser sends the cookie back, and keeps the same key.
        let again = connect_at(&svc, "/api", Some(&cookie)).await.unwrap();
        assert_eq!(again.cookie(), cookie);
        let profile = AuthFrontendClient::new(again.client.clone())
            .sign_in(&sign_in)
            .await
            .unwrap()
            .profile
            .unwrap();
        assert_eq!(profile.name, "alice");

        // Without it, the token doesn't open.
        let other = connect_ui(&svc).await;
        assert_ne!(other.cookie(), cookie);
        assert!(
            AuthFrontendClient::new(other.client.clone())
                .sign_in(&sign_in)
                .await
                .is_err()
        );
        svc.stop().await;
    });
}
