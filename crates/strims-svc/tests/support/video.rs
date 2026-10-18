// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The UI's and a player's side of a service's video: a signed-up profile
//! with its HLS calls, and raw HTTP requests of its HLS listener.

use std::net::SocketAddr;
use std::time::Duration as StdDuration;

use strims_proto::auth::v1::{AuthFrontendClient, SignUpRequest};
use strims_proto::video::v1::{
    HlsEgressClient, HlsEgressConfig, HlsEgressIsSupportedRequest, HlsEgressSetConfigRequest,
};
use strims_rpc::Client;
use strims_svc::{Config, Svc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::Instant;

use super::{Ui, connect_ui};

pub const STEP: StdDuration = StdDuration::from_secs(60);

/// Times the steps of a scenario.
pub struct Steps(pub Instant);

impl Steps {
    pub fn done(&mut self, step: &str) {
        let now = Instant::now();
        eprintln!("{step}: {:?}", now - self.0);
        self.0 = now;
    }
}

/// A profile signed up on a service: its UI connection and public key.
pub struct Profile {
    pub svc: Svc,
    pub ui: Ui,
    pub key: Vec<u8>,
}

impl Profile {
    pub async fn sign_up(config: Config) -> Profile {
        let svc = Svc::start(config).await.unwrap();
        let ui = connect_ui(&svc).await;
        let res = AuthFrontendClient::new(ui.client.clone())
            .sign_up(&SignUpRequest {
                name: "alice".into(),
                password: "pw".into(),
                ..Default::default()
            })
            .await
            .unwrap();
        let key = res.profile.unwrap().key.unwrap().public;
        Profile { svc, ui, key }
    }

    pub fn client(&self) -> Client {
        self.ui.client.clone()
    }

    pub fn hls(&self) -> HlsEgressClient<Client> {
        HlsEgressClient::new(self.client())
    }

    pub async fn hls_supported(&self) -> bool {
        self.hls()
            .is_supported(&HlsEgressIsSupportedRequest {})
            .await
            .unwrap()
            .supported
    }

    pub async fn set_hls(&self, enabled: bool) {
        self.hls()
            .set_config(&HlsEgressSetConfigRequest {
                config: Some(HlsEgressConfig {
                    enabled,
                    public_server_addr: String::new(),
                }),
            })
            .await
            .unwrap();
    }
}

/// An HTTP response.
pub struct Reply {
    pub status: u16,
    /// Names in lower case.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Reply {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    /// Asserts the headers every answer carries, and the content type and
    /// cache control given.
    pub fn check(&self, what: &str, content_type: &str, cache_control: &str) {
        assert_eq!(self.header("content-type"), Some(content_type), "{what}");
        assert_eq!(self.header("cache-control"), Some(cache_control), "{what}");
        assert_eq!(
            self.header("access-control-allow-origin"),
            Some("*"),
            "{what}"
        );
        assert_eq!(self.header("connection"), Some("close"), "{what}");
    }

    pub fn content_length(&self) -> usize {
        self.header("content-length").unwrap().parse().unwrap()
    }

    pub fn text(&self) -> String {
        String::from_utf8(self.body.clone()).unwrap()
    }
}

/// Sends a request of `path` to `addr` and reads the answer until the
/// server closes the connection.
pub async fn request(addr: SocketAddr, method: &str, path: &str) -> Reply {
    let mut s = TcpStream::connect(addr).await.unwrap();
    let req = format!("{method} {path} HTTP/1.1\r\nHost: {addr}\r\nAccept: */*\r\n\r\n");
    s.write_all(req.as_bytes()).await.unwrap();
    let mut raw = Vec::new();
    tokio::time::timeout(STEP, s.read_to_end(&mut raw))
        .await
        .expect("the server closes the connection")
        .unwrap();
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut head = httparse::Response::new(&mut headers);
    let httparse::Status::Complete(len) = head.parse(&raw).unwrap() else {
        panic!("a partial response: {}", String::from_utf8_lossy(&raw));
    };
    Reply {
        status: head.code.unwrap(),
        headers: head
            .headers
            .iter()
            .map(|h| {
                let value = String::from_utf8(h.value.to_vec()).unwrap();
                (h.name.to_ascii_lowercase(), value)
            })
            .collect(),
        body: raw[len..].to_vec(),
    }
}

/// A listed segment: its init's URI, EXTINF line and URI.
#[derive(Debug, Clone, PartialEq)]
pub struct Listed {
    pub init: String,
    pub extinf: String,
    pub uri: String,
}

pub fn listed(playlist: &str) -> Vec<Listed> {
    let mut out = Vec::new();
    let (mut init, mut extinf) = (String::new(), None);
    for line in playlist.lines() {
        if let Some(uri) = line.strip_prefix("#EXT-X-MAP:URI=") {
            init = uri.trim_matches('"').to_owned();
        } else if line.starts_with("#EXTINF:") {
            extinf = Some(line.to_owned());
        } else if !line.starts_with('#') {
            out.push(Listed {
                init: init.clone(),
                extinf: extinf.take().unwrap(),
                uri: line.to_owned(),
            });
        }
    }
    out
}
