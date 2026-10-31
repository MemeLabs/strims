// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The command line.

use std::net::IpAddr;
use std::path::PathBuf;

use strims_node::CaptureConfig;
use strims_node::app::PsiMode;
use strims_node::codec::FfmpegConfig;
use strims_svc::{Config, Db, TlsConfig};
use strims_video::codec::ffmpeg::FfmpegSettings;
use strims_video::codec::{MAX_RENDITIONS, RenditionPlan};

pub(crate) const USAGE: &str = "\
usage: strims-svc (--db <path> | --db-memory) [options]

Runs a profile's node, serves the UI's frontend RPC over WebSocket, and
serves the node's HLS streams to players over HTTP, or HTTPS with a
certificate.

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
  --psi <inline|thread>     where PSI sessions compute: on the node's
                            thread, or on their own (default thread)
  --max-connections <n>     UI connections served at once (default 64)
  --hls-listen <host:port>  where players fetch HLS over HTTP; without it,
                            HLS egress is unsupported
  --hls-public-url <url>    the http(s):// URL playlist URLs start with when
                            the HLS config sets no public address (default
                            http:// and the HLS address; for an unspecified
                            address, the public IP or hostname at its port;
                            with --tls-cert, https:// and the first of those
                            the certificate names, else its DNS name)
  --tls-cert <path>         serve HLS over HTTPS (HTTP/2 or HTTP/1.1) with
                            the certificate chain in this PEM file, the
                            service's own first
  --tls-key <path>          the PEM file of the certificate's private key
  --api-tls                 the API takes wss:// connections, with the
                            --tls-cert certificate
  --ffmpeg <path>           the ffmpeg (7.1 or later, with libx264) the node
                            transcodes with: it turns HEVC and AV1 publishes
                            into the H.264 stream every viewer can play
  --renditions <list>       with --ffmpeg, the renditions made of every
                            stream below its shorter side, each another
                            swarm the node seeds: comma-separated
                            <side>p<fps>@<bitrate>, such as
                            720p60@3000k,480p30@1200k (default none)
  --ppspp-capture <dir>     capture the node's ppspp frames and scheduler
                            decisions into <dir> (`ppspp-capture` reads them)
  --ppspp-capture-max <MiB> the most a capture keeps, in its last 8 files
                            (default 1024)
  --help                    print this

Players accept a certificate that chains to a root their device trusts
and names the playlist URL's host. A DNS name's certificate from a public
CA plays everywhere, on AirPlay and Cast receivers too: ACME's DNS-01
challenge (certbot, lego, acme.sh) issues one for a name a public DNS
record points at a LAN address. The service checks both files each minute
and serves a renewal without a restart.

The log level comes from RUST_LOG (default info).";

/// What the command line asks for.
pub(crate) enum Command {
    Run(Box<Config>),
    Help,
}

/// The renditions `--renditions` lists: `<side>p<fps>@<bitrate>`, the
/// bitrate in bits a second, or thousands of them with `k`, or millions
/// with `M`.
fn ladder(list: &str) -> Result<Vec<RenditionPlan>, String> {
    let bad =
        |r: &str| format!("--renditions takes <side>p<fps>@<bitrate>, such as 480p30@1200k: {r}");
    let rungs = list
        .split(',')
        .map(|r| {
            let (size, bitrate) = r.split_once('@').ok_or_else(|| bad(r))?;
            let (side, fps) = size.split_once('p').ok_or_else(|| bad(r))?;
            let (digits, scale) = match bitrate.strip_suffix(['k', 'K']) {
                Some(d) => (d, 1_000),
                None => match bitrate.strip_suffix('M') {
                    Some(d) => (d, 1_000_000),
                    None => (bitrate, 1),
                },
            };
            let plan = RenditionPlan {
                short_side: side.parse().map_err(|_| bad(r))?,
                max_frame_rate: fps.parse().map_err(|_| bad(r))?,
                bitrate: digits
                    .parse::<u64>()
                    .ok()
                    .and_then(|d| d.checked_mul(scale))
                    .ok_or_else(|| bad(r))?,
            };
            if plan.short_side == 0 || plan.max_frame_rate == 0 || plan.bitrate == 0 {
                return Err(bad(r));
            }
            Ok(plan)
        })
        .collect::<Result<Vec<_>, String>>()?;
    if rungs.len() >= MAX_RENDITIONS {
        return Err(format!(
            "--renditions lists at most {} renditions",
            MAX_RENDITIONS - 1
        ));
    }
    Ok(rungs)
}

/// Parses the arguments after the program name. Flags take their value as
/// the next argument or after `=`.
pub(crate) fn parse(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut config = Config::default();
    let mut db = None;
    let mut capture_dir = None;
    let mut capture_max = None;
    let mut tls_cert = None;
    let mut tls_key = None;
    let mut api_tls = false;
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
            "--psi" => {
                app.psi = match value()?.as_str() {
                    "inline" => PsiMode::Inline,
                    "thread" => PsiMode::Thread,
                    v => return Err(format!("--psi must be inline or thread: {v}")),
                }
            }
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
            "--tls-cert" => tls_cert = Some(PathBuf::from(value()?)),
            "--tls-key" => tls_key = Some(PathBuf::from(value()?)),
            "--api-tls" => api_tls = true,
            "--ffmpeg" => {
                app.ffmpeg = Some(FfmpegConfig {
                    program: PathBuf::from(value()?),
                    settings: FfmpegSettings::default(),
                })
            }
            "--renditions" => app.video.ingress.transcode.ladder = ladder(&value()?)?,
            "--ppspp-capture" => capture_dir = Some(PathBuf::from(value()?)),
            "--ppspp-capture-max" => capture_max = Some(positive::<u64>(&flag, &value()?)?),
            _ => return Err(format!("unknown argument {flag}")),
        }
    }
    config.db = db.ok_or("pass --db <path> or --db-memory")?;
    match (capture_dir, capture_max) {
        (Some(dir), max) => {
            config.app.ppspp_capture = Some(CaptureConfig {
                dir,
                max_bytes: max.unwrap_or(DEFAULT_CAPTURE_MIB).saturating_mul(1 << 20),
                tap: true,
            });
        }
        (None, Some(_)) => return Err("--ppspp-capture-max needs --ppspp-capture".to_owned()),
        (None, None) => {}
    }
    if config.hls_listen.is_none() && !config.app.video.hls.base_url.is_empty() {
        return Err("--hls-public-url needs --hls-listen".to_owned());
    }
    config.tls = match (tls_cert, tls_key) {
        (Some(cert), Some(key)) => {
            if config.hls_listen.is_none() && !api_tls {
                return Err("--tls-cert serves --hls-listen, or the API with --api-tls".to_owned());
            }
            Some(TlsConfig {
                api: api_tls,
                ..TlsConfig::new(cert, key)
            })
        }
        (None, None) if api_tls => return Err("--api-tls needs --tls-cert".to_owned()),
        (None, None) => None,
        _ => return Err("--tls-cert and --tls-key go together".to_owned()),
    };
    Ok(Command::Run(Box::new(config)))
}

/// What a ppspp capture keeps without `--ppspp-capture-max`, in MiB.
const DEFAULT_CAPTURE_MIB: u64 = 1024;

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
            "--psi=inline",
            "--max-connections",
            "8",
            "--hls-listen",
            "0.0.0.0:8082",
            "--hls-public-url=https://media.example",
            "--ppspp-capture",
            "/tmp/cap",
            "--ppspp-capture-max=16",
            "--ffmpeg=/opt/ffmpeg/bin/ffmpeg",
            "--tls-cert",
            "/etc/strims/fullchain.pem",
            "--tls-key=/etc/strims/privkey.pem",
            "--api-tls",
        ])
        .unwrap();
        assert_eq!(
            c.tls,
            Some(TlsConfig {
                api: true,
                ..TlsConfig::new("/etc/strims/fullchain.pem", "/etc/strims/privkey.pem")
            })
        );
        assert!(c.app.video.ingress.transcode.ladder.is_empty());
        let ffmpeg = c.app.ffmpeg.as_ref().unwrap();
        assert_eq!(ffmpeg.program.to_str(), Some("/opt/ffmpeg/bin/ffmpeg"));
        assert_eq!(ffmpeg.settings, FfmpegSettings::default());
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
        assert_eq!(c.app.psi, PsiMode::Inline);
        assert_eq!(c.max_connections, 8);
        assert_eq!(c.hls_listen.as_deref(), Some("0.0.0.0:8082"));
        assert_eq!(c.app.video.hls.base_url, "https://media.example");
        assert_eq!(
            c.app.ppspp_capture,
            Some(CaptureConfig {
                dir: "/tmp/cap".into(),
                max_bytes: 16 << 20,
                tap: true,
            })
        );
        let c = run(&["--db-memory", "--ppspp-capture=/tmp/cap"]).unwrap();
        assert_eq!(
            c.app.ppspp_capture.map(|c| c.max_bytes),
            Some(DEFAULT_CAPTURE_MIB << 20)
        );
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
        assert_eq!(c.app.psi, PsiMode::Thread);
        assert_eq!(c.hls_listen, None);
        assert_eq!(c.app.video.hls.base_url, "");
        assert_eq!(c.app.ppspp_capture, None);
        assert_eq!(c.tls, None);
        let c = run(&[
            "--db-memory",
            "--hls-listen=:0",
            "--tls-cert=c",
            "--tls-key=k",
        ])
        .unwrap();
        let tls = c.tls.unwrap();
        assert!(!tls.api);
        assert_eq!(tls.reload_interval, strims_svc::DEFAULT_TLS_RELOAD_INTERVAL);
    }

    #[test]
    fn renditions_list_their_sides_rates_and_bitrates() {
        let c = run(&[
            "--db-memory",
            "--renditions",
            "720p60@3M,480p30@1200k,240p15@250000",
        ])
        .unwrap();
        let rung = |short_side, max_frame_rate, bitrate| RenditionPlan {
            short_side,
            max_frame_rate,
            bitrate,
        };
        assert_eq!(
            c.app.video.ingress.transcode.ladder,
            [
                rung(720, 60, 3_000_000),
                rung(480, 30, 1_200_000),
                rung(240, 15, 250_000)
            ]
        );
        for bad in [
            "720p60",
            "720@3M",
            "p30@1M",
            "720p0@1M",
            "720p30@",
            "720p30@1G",
            "x",
        ] {
            let e = run(&["--db-memory", "--renditions", bad]).err().unwrap();
            assert!(e.contains("<side>p<fps>@<bitrate>"), "{bad}: {e}");
        }
        let eight = ["240p30@1M"; MAX_RENDITIONS].join(",");
        let e = run(&["--db-memory", "--renditions", &eight]).err().unwrap();
        assert!(e.contains("at most 7"), "{e}");
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
        assert!(run(&["--db-memory", "--psi", "worker"]).is_err());
        assert!(run(&["--db-memory", "--bogus"]).is_err());
        let max = run(&["--db-memory", "--ppspp-capture-max", "8"]);
        assert!(max.err().unwrap().contains("needs --ppspp-capture"));
        assert!(
            run(&[
                "--db-memory",
                "--ppspp-capture=/tmp/cap",
                "--ppspp-capture-max=0"
            ])
            .is_err()
        );
        for url in ["media.example", "ftp://media.example", "http://"] {
            let args = ["--db-memory", "--hls-listen=:0", "--hls-public-url", url];
            assert!(run(&args).err().unwrap().contains("http(s)://"), "{url}");
        }
        let lone = run(&["--db-memory", "--hls-public-url", "HTTP://a"]);
        assert!(lone.err().unwrap().contains("--hls-listen"));
        for (args, why) in [
            (&["--hls-listen=:0", "--tls-cert=c"][..], "go together"),
            (&["--hls-listen=:0", "--tls-key=k"], "go together"),
            (&["--hls-listen=:0", "--api-tls"], "--api-tls needs"),
            (&["--tls-cert=c", "--tls-key=k"], "serves --hls-listen"),
        ] {
            let args = [&["--db-memory"][..], args].concat();
            let e = run(&args).err().unwrap();
            assert!(e.contains(why), "{args:?}: {e}");
        }
        let api_only = run(&["--db-memory", "--tls-cert=c", "--tls-key=k", "--api-tls"]);
        assert!(api_only.unwrap().tls.unwrap().api);
        assert!(matches!(
            parse(["--help".to_owned()]).unwrap(),
            Command::Help
        ));
    }
}
