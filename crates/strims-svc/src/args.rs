// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The command line.

use std::net::IpAddr;
use std::path::PathBuf;

use strims_svc::{Config, Db};

pub(crate) const USAGE: &str = "\
usage: strims-svc (--db <path> | --db-memory) [options]

Runs a profile's node, serves the UI's frontend RPC over WebSocket, and
serves the node's HLS streams to players over plain HTTP.

options:
  --db <path>               the redb file holding the profiles' stores
  --db-memory               keep the stores in memory
  --api-listen <host:port>  where the UI connects (default 127.0.0.1:8083)
  --api-path <path>         the API's WebSocket path (default /api)
  --tcp-listen <host:port>  where the node accepts TCP links
  --ws-listen <host:port>   where the node accepts WebSocket links
  --public-ip <ip>          the address link URIs advertise for listeners
                            on an unspecified address
  --public-hostname <host>  the host WebSocket link URIs advertise
  --ws-public-port <port>   the port WebSocket link URIs advertise
  --allow-insecure-ws       dial ws:// links, which TLS doesn't protect
  --max-peers <n>           the most peers the node takes, whatever the
                            profile's vnic settings allow
  --max-upload <bytes/s>    the most bytes per second the node uploads,
                            whatever the profile's vnic settings allow
  --label <name>            the node's host certificate subject
  --max-connections <n>     UI connections served at once (default 64)
  --hls-listen <host:port>  where players fetch HLS over HTTP; without it,
                            HLS egress is unsupported
  --hls-public-url <url>    the http(s):// URL playlist URLs start with when
                            the HLS config sets no public address (default
                            http:// and the HLS address; for an unspecified
                            address, the public IP or hostname at its port)
  --help                    print this

The log level comes from RUST_LOG (default info).";

/// What the command line asks for.
pub(crate) enum Command {
    Run(Box<Config>),
    Help,
}

/// Parses the arguments after the program name. Flags take their value as
/// the next argument or after `=`.
pub(crate) fn parse(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut config = Config::default();
    let mut db = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) => (f.to_owned(), Some(v.to_owned())),
            None => (arg, None),
        };
        let mut value = || {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        let app = &mut config.app;
        match flag.as_str() {
            "--help" | "-h" => return Ok(Command::Help),
            "--db" => db = Some(Db::File(PathBuf::from(value()?))),
            "--db-memory" => db = Some(Db::Memory),
            "--api-listen" => config.api_listen = value()?,
            "--api-path" => {
                let path = value()?;
                if !path.starts_with('/') {
                    return Err(format!("--api-path must start with /: {path}"));
                }
                config.api_path = path;
            }
            "--tcp-listen" => app.tcp_listen = Some(value()?),
            "--ws-listen" => app.ws_listen = Some(value()?),
            "--public-ip" => app.public_ip = Some(number::<IpAddr>(&flag, &value()?)?),
            "--public-hostname" => app.ws_public_hostname = value()?,
            "--ws-public-port" => app.ws_public_port = number(&flag, &value()?)?,
            "--allow-insecure-ws" => app.allow_insecure_ws = true,
            "--max-peers" => app.max_peers = Some(positive(&flag, &value()?)?),
            "--max-upload" => app.max_upload = Some(positive(&flag, &value()?)?),
            "--label" => app.label = value()?,
            "--max-connections" => config.max_connections = number(&flag, &value()?)?,
            "--hls-listen" => config.hls_listen = Some(value()?),
            "--hls-public-url" => {
                let url = value()?;
                if !is_http_url(&url) {
                    return Err(format!(
                        "--hls-public-url must start with http(s)://: {url}"
                    ));
                }
                app.video.hls.base_url = url;
            }
            _ => return Err(format!("unknown argument {flag}")),
        }
    }
    config.db = db.ok_or("pass --db <path> or --db-memory")?;
    if config.hls_listen.is_none() && !config.app.video.hls.base_url.is_empty() {
        return Err("--hls-public-url needs --hls-listen".to_owned());
    }
    Ok(Command::Run(Box::new(config)))
}

fn number<T: std::str::FromStr>(flag: &str, v: &str) -> Result<T, String> {
    v.parse().map_err(|_| format!("{flag}: invalid value {v}"))
}

/// `v` as a number other than 0, since a vnic limit of 0 is unlimited.
fn positive<T: std::str::FromStr + Default + PartialEq>(flag: &str, v: &str) -> Result<T, String> {
    let n = number(flag, v)?;
    if n == T::default() {
        return Err(format!("{flag} must be at least 1"));
    }
    Ok(n)
}

/// Whether `url` is `http://` or `https://`, in any case, and more.
fn is_http_url(url: &str) -> bool {
    ["http://", "https://"].iter().any(|scheme| {
        url.len() > scheme.len()
            && url
                .get(..scheme.len())
                .is_some_and(|p| p.eq_ignore_ascii_case(scheme))
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use super::*;

    fn run(args: &[&str]) -> Result<Config, String> {
        match parse(args.iter().map(|a| (*a).to_owned()))? {
            Command::Run(c) => Ok(*c),
            Command::Help => panic!("help"),
        }
    }

    #[test]
    fn flags_take_values_after_a_space_or_an_equals_sign() {
        let c = run(&[
            "--db=/tmp/a.redb",
            "--api-listen",
            "0.0.0.0:9000",
            "--api-path=/manage",
            "--tcp-listen",
            "0.0.0.0:9001",
            "--ws-listen=0.0.0.0:9002",
            "--public-ip",
            "10.0.0.1",
            "--public-hostname=node.example",
            "--ws-public-port",
            "443",
            "--allow-insecure-ws",
            "--max-peers=4",
            "--max-upload",
            "1000000",
            "--label",
            "a",
            "--max-connections",
            "8",
            "--hls-listen",
            "0.0.0.0:8082",
            "--hls-public-url=https://media.example",
        ])
        .unwrap();
        assert!(matches!(&c.db, Db::File(p) if p.to_str() == Some("/tmp/a.redb")));
        assert_eq!(c.api_listen, "0.0.0.0:9000");
        assert_eq!(c.api_path, "/manage");
        assert_eq!(c.app.tcp_listen.as_deref(), Some("0.0.0.0:9001"));
        assert_eq!(c.app.ws_listen.as_deref(), Some("0.0.0.0:9002"));
        assert_eq!(c.app.public_ip, Some("10.0.0.1".parse().unwrap()));
        assert_eq!(c.app.ws_public_hostname, "node.example");
        assert_eq!(c.app.ws_public_port, 443);
        assert!(c.app.allow_insecure_ws);
        assert_eq!(c.app.max_peers, Some(4));
        assert_eq!(c.app.max_upload, Some(1_000_000));
        assert_eq!(c.app.label, "a");
        assert_eq!(c.max_connections, 8);
        assert_eq!(c.hls_listen.as_deref(), Some("0.0.0.0:8082"));
        assert_eq!(c.app.video.hls.base_url, "https://media.example");
    }

    #[test]
    fn defaults_hold_without_flags() {
        let c = run(&["--db-memory"]).unwrap();
        assert!(matches!(c.db, Db::Memory));
        assert_eq!(c.api_listen, "127.0.0.1:8083");
        assert_eq!(c.api_path, "/api");
        assert_eq!(c.app.tcp_listen, None);
        assert!(!c.app.allow_insecure_ws);
        assert_eq!(c.app.max_peers, None);
        assert_eq!(c.app.max_upload, None);
        assert_eq!(c.hls_listen, None);
        assert_eq!(c.app.video.hls.base_url, "");
    }

    #[test]
    fn bad_command_lines_are_refused() {
        assert!(run(&[]).err().unwrap().contains("--db"));
        assert!(run(&["--db-memory", "--max-peers", "x"]).is_err());
        assert!(run(&["--db-memory", "--max-upload=-1"]).is_err());
        for flag in ["--max-peers", "--max-upload"] {
            let zero = run(&["--db-memory", flag, "0"]);
            assert!(zero.err().unwrap().contains("at least 1"), "{flag}");
        }
        assert!(run(&["--db-memory", "--public-ip", "nope"]).is_err());
        assert!(run(&["--db-memory", "--api-path", "api"]).is_err());
        assert!(run(&["--db-memory", "--tcp-listen"]).is_err());
        assert!(run(&["--db-memory", "--bogus"]).is_err());
        for url in ["media.example", "ftp://media.example", "http://"] {
            let args = ["--db-memory", "--hls-listen=:0", "--hls-public-url", url];
            assert!(run(&args).err().unwrap().contains("http(s)://"), "{url}");
        }
        let lone = run(&["--db-memory", "--hls-public-url", "HTTP://a"]);
        assert!(lone.err().unwrap().contains("--hls-listen"));
        assert!(matches!(
            parse(["--help".to_owned()]).unwrap(),
            Command::Help
        ));
    }
}
