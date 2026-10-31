// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A service on localhost with the operator's certificate: HLS over HTTPS,
//! HTTP/1.1 or HTTP/2, at an `https://` base URL, the API over `wss://`, a
//! renewal served to new connections without a restart, and certificates
//! that don't load refused at the start.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use std::time::Duration as StdDuration;

use strims_ppspp::{SwarmId, Uri};
use strims_proto::auth::v1::{AuthFrontendClient, SignUpRequest};
use strims_proto::network::v1::directory::ListingSnippet;
use strims_proto::network::v1::{CreateServerRequest, ListNetworksRequest, NetworkFrontendClient};
use strims_proto::video::v1::{
    HlsEgressClient, HlsEgressConfig, HlsEgressIsSupportedRequest, HlsEgressOpenStreamRequest,
    HlsEgressSetConfigRequest, VideoChannelCreateRequest, VideoChannelFrontendClient,
};
use strims_svc::{Config, Svc, SvcError, TlsConfig, TlsError};
use strims_video::{ingress_options, units};
use support::tls::*;
use support::video::STEP;
use support::*;

/// A service of [`local_config`] serving HLS, with `tls`.
fn tls_config(tls: TlsConfig) -> Config {
    Config {
        hls_listen: Some("127.0.0.1:0".into()),
        tls: Some(tls),
        ..local_config()
    }
}

#[test]
fn hls_and_the_api_serve_with_the_operator_certificate() {
    run(async {
        let dir = dir("serve");
        let cert = TestCert::new(&["localhost", "127.0.0.1"]);
        let (cert_path, key_path) = cert.write(&dir, 0);
        let svc = Svc::start(tls_config(TlsConfig {
            api: true,
            ..TlsConfig::new(&cert_path, &key_path)
        }))
        .await
        .unwrap();
        let hls_addr = svc.hls_addr().unwrap();
        assert_eq!(svc.hls_base_url(), format!("https://{hls_addr}"));

        // The API takes wss:// alone, from clients that trust the
        // certificate.
        assert!(connect_ui_at_ws(&svc).await.is_err());
        let other = TestCert::new(&["localhost", "127.0.0.1"]);
        assert!(
            connect_wss(&svc, &other.client(&[]), "localhost")
                .await
                .is_err()
        );
        let client = cert.client(&[]);
        let ui = connect_wss(&svc, &client, "localhost").await.unwrap();
        let set = ui.set_cookie.as_deref().unwrap();
        assert!(set.starts_with("session_key="), "{set}");
        AuthFrontendClient::new(ui.client.clone())
            .sign_up(&SignUpRequest {
                name: "alice".into(),
                password: "pw".into(),
                ..Default::default()
            })
            .await
            .unwrap();

        let network = NetworkFrontendClient::new(ui.client.clone())
            .create_server(&CreateServerRequest {
                name: "streams".into(),
                ..Default::default()
            })
            .await
            .unwrap()
            .network
            .unwrap();
        let net = network_key(network.certificate.as_ref().unwrap());
        let channel = VideoChannelFrontendClient::new(ui.client.clone())
            .create(&VideoChannelCreateRequest {
                directory_listing_snippet: Some(ListingSnippet {
                    title: "nothing yet".into(),
                    ..Default::default()
                }),
                network_key: net.clone(),
            })
            .await
            .unwrap()
            .channel
            .unwrap();
        let hls = HlsEgressClient::new(ui.client.clone());
        assert!(
            hls.is_supported(&HlsEgressIsSupportedRequest {})
                .await
                .unwrap()
                .supported
        );
        hls.set_config(&HlsEgressSetConfigRequest {
            config: Some(HlsEgressConfig {
                enabled: true,
                public_server_addr: String::new(),
            }),
        })
        .await
        .unwrap();

        // A stream's playlist URL is https://, and its playlist answers
        // over TLS, at the address and as its certificate's DNS name.
        let channel_key = channel.key.unwrap().public;
        let uri = Uri::new(SwarmId::new(&channel_key), ingress_options().uri_options());
        let playlist_url = hls
            .open_stream(&HlsEgressOpenStreamRequest {
                swarm_uri: uri.to_string(),
                network_keys: vec![net],
                mime_type: units::MIME_TYPE.into(),
            })
            .await
            .unwrap()
            .playlist_url;
        let path = playlist_url
            .strip_prefix(&format!("https://{hls_addr}"))
            .unwrap_or_else(|| panic!("{playlist_url}"));
        for name in ["127.0.0.1", "localhost"] {
            let r = request(&client, hls_addr, name, "GET", path).await;
            assert_eq!(r.status, 200, "{}", r.text());
            r.check("the playlist", "application/vnd.apple.mpegurl", "no-cache");
            assert!(r.text().starts_with("#EXTM3U\n"), "{}", r.text());
        }
        let r = request(&client, hls_addr, "localhost", "GET", "/nope").await;
        assert_eq!(r.status, 404);

        // A client that offers HTTP/2 takes it, for any number of requests
        // on one connection.
        let h2 = cert.client(&[b"h2", b"http/1.1"]);
        let replies = request_h2(&h2, hls_addr, "localhost", &[path, path, "/nope"]).await;
        let playlist = request(&client, hls_addr, "localhost", "GET", path).await;
        for r in &replies[..2] {
            assert_eq!(r.status, 200, "{}", r.text());
            for name in [
                "content-type",
                "cache-control",
                "access-control-allow-origin",
            ] {
                assert_eq!(r.header(name), playlist.header(name), "{name}");
            }
            assert_eq!(r.header("connection"), None);
            assert_eq!(r.content_length(), r.body.len());
            assert_eq!(r.text(), playlist.text());
        }
        assert_eq!(replies[2].status, 404);
        assert!(
            connect(&other.client(&[]), hls_addr, "localhost")
                .await
                .is_err()
        );

        svc.stop().await;
        let _ = std::fs::remove_dir_all(&dir);
    });
}

/// Connects to `svc`'s API without TLS.
async fn connect_ui_at_ws(svc: &Svc) -> Result<Ui, tokio_tungstenite::tungstenite::Error> {
    tokio::time::timeout(STEP, connect_at(svc, "/api", None))
        .await
        .expect("the API ends a plaintext upgrade")
}

#[test]
fn a_renewed_certificate_serves_new_connections_without_a_restart() {
    run(async {
        let dir = dir("renew");
        let first = TestCert::new(&["localhost"]);
        let (cert_path, key_path) = first.write(&dir, 0);
        let svc = Svc::start(tls_config(TlsConfig {
            api: true,
            reload_interval: StdDuration::from_millis(50),
            ..TlsConfig::new(&cert_path, &key_path)
        }))
        .await
        .unwrap();
        let hls_addr = svc.hls_addr().unwrap();
        assert_eq!(
            svc.hls_base_url(),
            format!("https://localhost:{}", hls_addr.port())
        );
        let ui = connect_wss(&svc, &first.client(&[]), "localhost")
            .await
            .unwrap();
        AuthFrontendClient::new(ui.client.clone())
            .sign_up(&SignUpRequest {
                name: "alice".into(),
                password: "pw".into(),
                ..Default::default()
            })
            .await
            .unwrap();

        let renewed = TestCert::new(&["localhost"]);
        renewed.write(&dir, 1);
        let client = renewed.client(&[]);
        eventually("the renewed certificate", STEP, || async {
            connect(&client, hls_addr, "localhost").await.ok()
        })
        .await;
        assert!(
            connect(&first.client(&[]), hls_addr, "localhost")
                .await
                .is_err()
        );
        assert!(connect_wss(&svc, &client, "localhost").await.is_ok());
        // The API connection from before the renewal goes on.
        let networks = NetworkFrontendClient::new(ui.client.clone())
            .list(&ListNetworksRequest {})
            .await;
        assert!(networks.is_ok());

        // A pair that doesn't load leaves the renewed one serving.
        std::fs::write(&key_path, &first.key_pem).unwrap();
        tokio::time::sleep(StdDuration::from_millis(200)).await;
        assert!(connect(&client, hls_addr, "localhost").await.is_ok());

        svc.stop().await;
        let _ = std::fs::remove_dir_all(&dir);
    });
}

#[test]
fn certificates_that_dont_load_fail_the_start() {
    run(async {
        let dir = dir("refused");
        let cert = TestCert::new(&["localhost"]);
        let (cert_path, key_path) = cert.write(&dir, 0);
        let missing = dir.join("missing.pem");
        let Err(e) = Svc::start(tls_config(TlsConfig::new(&missing, &key_path))).await else {
            panic!("the service started without its certificate");
        };
        assert!(
            matches!(e, SvcError::Tls(TlsError::Certificates { .. })),
            "{e}"
        );
        std::fs::write(&key_path, TestCert::new(&["localhost"]).key_pem).unwrap();
        let Err(e) = Svc::start(tls_config(TlsConfig::new(&cert_path, &key_path))).await else {
            panic!("the service started with another certificate's key");
        };
        assert!(matches!(e, SvcError::Tls(TlsError::Rejected(_))), "{e}");
        let _ = std::fs::remove_dir_all(&dir);
    });
}
