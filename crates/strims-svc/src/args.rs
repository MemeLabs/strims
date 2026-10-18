// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The command line.

use std::net::IpAddr;
use std::path::PathBuf;

use strims_svc::{Config, Db};

pub(crate) const USAGE: &str = "\
usage: strims-svc (--db <path> | --db-memory) [options]

Runs a profile's node and serves the UI's frontend RPC over WebSocket.

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
  --max-peers <n>           the node's peer budget (default 16)
  --label <name>            the node's host certificate subject
  --max-connections <n>     UI connections served at once (default 64)
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
            "--max-peers" => app.max_peers = number(&flag, &value()?)?,
            "--label" => app.label = value()?,
            "--max-connections" => config.max_connections = number(&flag, &value()?)?,
            _ => return Err(format!("unknown argument {flag}")),
        }
    }
    config.db = db.ok_or("pass --db <path> or --db-memory")?;
    Ok(Command::Run(Box::new(config)))
}

fn number<T: std::str::FromStr>(flag: &str, v: &str) -> Result<T, String> {
    v.parse().map_err(|_| format!("{flag}: invalid value {v}"))
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
            "--label",
            "a",
            "--max-connections",
            "8",
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
        assert_eq!(c.app.max_peers, 4);
        assert_eq!(c.app.label, "a");
        assert_eq!(c.max_connections, 8);
    }

    #[test]
    fn defaults_hold_without_flags() {
        let c = run(&["--db-memory"]).unwrap();
        assert!(matches!(c.db, Db::Memory));
        assert_eq!(c.api_listen, "127.0.0.1:8083");
        assert_eq!(c.api_path, "/api");
        assert_eq!(c.app.tcp_listen, None);
        assert!(!c.app.allow_insecure_ws);
        assert_eq!(c.app.max_peers, 16);
    }

    #[test]
    fn bad_command_lines_are_refused() {
        assert!(run(&[]).err().unwrap().contains("--db"));
        assert!(run(&["--db-memory", "--max-peers", "x"]).is_err());
        assert!(run(&["--db-memory", "--public-ip", "nope"]).is_err());
        assert!(run(&["--db-memory", "--api-path", "api"]).is_err());
        assert!(run(&["--db-memory", "--tcp-listen"]).is_err());
        assert!(run(&["--db-memory", "--bogus"]).is_err());
        assert!(matches!(
            parse(["--help".to_owned()]).unwrap(),
            Command::Help
        ));
    }
}
