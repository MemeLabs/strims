// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The command line as a person or a script runs it: the built binary, on
//! a store of its own, then against a service running on that store.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};

use serde_json::Value;
use strims_svc::{Config, Db, Svc};
use tokio::process::Command;
use tokio::task::LocalSet;

/// What a run of the binary printed, and how it exited.
struct Out {
    code: i32,
    stdout: String,
    stderr: String,
}

impl Out {
    #[track_caller]
    fn ok(self) -> String {
        assert_eq!(self.code, 0, "stderr: {}", self.stderr);
        self.stdout
    }

    #[track_caller]
    fn json(self) -> Value {
        serde_json::from_str(&self.ok()).unwrap()
    }

    #[track_caller]
    fn fails(self, code: i32, says: &str) {
        assert_eq!(
            self.code, code,
            "stdout: {} stderr: {}",
            self.stdout, self.stderr
        );
        assert!(
            self.stderr.contains(says),
            "{says:?} not in {}",
            self.stderr
        );
    }
}

async fn run(args: &[&str]) -> Out {
    let out = Command::new(env!("CARGO_BIN_EXE_strims-svc"))
        .args(args)
        .output()
        .await
        .unwrap();
    Out {
        code: out.status.code().unwrap(),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
    }
}

/// A store's path and a password file, in a directory of their own.
struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("strims-cli-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("pw"), "a password\n").unwrap();
        std::fs::write(dir.join("wrong"), "not it\n").unwrap();
        Scratch { dir }
    }

    fn path(&self, name: &str) -> String {
        self.dir.join(name).to_str().unwrap().to_owned()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn local<F: std::future::Future<Output = ()>>(f: F) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    LocalSet::new().block_on(&rt, f);
}

/// The networks `strims-svc call ... NetworkFrontend.List` printed, by
/// name.
fn network_names(list: &Value) -> Vec<String> {
    list["networks"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|n| n["server_config"]["name"].as_str().unwrap_or("").to_owned())
                .collect()
        })
        .unwrap_or_default()
}

async fn service_on(db: &Path) -> Svc {
    Svc::start(Config {
        api_listen: "127.0.0.1:0".into(),
        db: Db::File(db.to_owned()),
        ..Config::default()
    })
    .await
    .unwrap()
}

#[test]
fn a_profile_is_added_called_on_exported_removed_and_imported_from_the_command_line() {
    local(async {
        let s = Scratch::new("crud");
        let (db, pw) = (s.path("strims.redb"), s.path("pw"));
        let login = ["--profile", "alice", "--profile-password-file", pw.as_str()];
        let on_db = |args: &[&str]| -> Vec<String> {
            let mut v: Vec<String> = ["call", "--db", db.as_str()]
                .into_iter()
                .chain(login)
                .map(str::to_owned)
                .collect();
            v.extend(args.iter().map(|a| (*a).to_owned()));
            v
        };
        let call = |args: Vec<String>| async move {
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            run(&args).await
        };

        run(&[
            "profiles",
            "add",
            "--db",
            &db,
            "--profile",
            "alice",
            "--profile-password-file",
            &pw,
        ])
        .await
        .ok();
        assert_eq!(
            run(&["profiles", "list", "--db", &db]).await.ok(),
            "alice\n"
        );

        let created = call(on_db(&[
            "NetworkFrontend.CreateServer",
            r#"{"name": "streams"}"#,
        ]))
        .await
        .json();
        let network = &created["network"];
        let id = network["id"].as_str().unwrap().to_owned();
        let root = network["certificate"]["parent"]["body"].as_str().unwrap();
        let body = run(&["decode", "strims.type.CertificateBody", root])
            .await
            .json();
        assert_eq!(body["subject"], "streams");
        let key = body["key"].as_str().unwrap().to_owned();

        let list = call(on_db(&["NetworkFrontend.List"])).await.json();
        assert_eq!(network_names(&list), ["streams"]);

        let invitation = call(on_db(&[
            "NetworkFrontend.CreateInvitation",
            &format!(r#"{{"network_id": "{id}"}}"#),
        ]))
        .await
        .json();
        let data = invitation["invitation"]["data"].as_str().unwrap();
        let v0 = run(&["decode", "strims.network.v1.InvitationV0", data])
            .await
            .json();
        assert!(v0["certificate"]["body"].is_string(), "{v0}");

        let channel = format!(
            r#"{{"network_key": "{key}", "directory_listing_snippet": {{"title": "one click"}}}}"#
        );
        call(on_db(&["VideoChannelFrontend.Create", &channel]))
            .await
            .ok();
        let channels = call(on_db(&["VideoChannelFrontend.List"])).await.ok();
        assert!(channels.contains("one click"), "{channels}");

        let profile = call(on_db(&["ProfileFrontend.Get"])).await.json();
        assert_eq!(profile["profile"]["name"], "alice");

        // a service on the store holds it; the command line reaches it by
        // its API
        let svc = service_on(Path::new(&db)).await;
        let api = format!("ws://{}/api", svc.api_addr());
        call(on_db(&["NetworkFrontend.List"]))
            .await
            .fails(1, "--api");
        let over_api = |args: &[&str]| -> Vec<String> {
            let mut v: Vec<String> = ["call", "--api", api.as_str()]
                .into_iter()
                .chain(login)
                .map(str::to_owned)
                .collect();
            v.extend(args.iter().map(|a| (*a).to_owned()));
            v
        };
        let list = call(over_api(&["NetworkFrontend.List"])).await.json();
        assert_eq!(network_names(&list), ["streams"]);
        let watched = call(over_api(&[
            "NetworkFrontend.Watch",
            "--limit",
            "1",
            "--timeout",
            "30",
        ]))
        .await
        .ok();
        assert_eq!(watched.lines().count(), 1, "{watched}");
        serde_json::from_str::<Value>(watched.trim()).unwrap();
        svc.stop().await;

        let export = s.path("alice.export");
        let profile_args = |action: &str, pw: &str| -> Vec<String> {
            [
                "profiles",
                action,
                "--db",
                db.as_str(),
                "--profile",
                "alice",
                "--profile-password-file",
                pw,
            ]
            .into_iter()
            .map(str::to_owned)
            .collect()
        };
        let wrong = s.path("wrong");
        call(profile_args("remove", &wrong))
            .await
            .fails(1, "credentials");
        let mut export_args = profile_args("export", &pw);
        export_args.extend(["--out".to_owned(), export.clone()]);
        call(export_args.clone()).await.ok();
        // never over a file
        call(export_args).await.fails(1, "writing");
        call(profile_args("remove", &pw)).await.ok();
        assert_eq!(run(&["profiles", "list", "--db", &db]).await.ok(), "");
        call(on_db(&["NetworkFrontend.List"]))
            .await
            .fails(1, "signing in as alice");

        let import = [
            "profiles",
            "import",
            "--db",
            db.as_str(),
            "--in",
            export.as_str(),
        ];
        assert_eq!(run(&import).await.ok(), "alice\n");
        let list = call(on_db(&["NetworkFrontend.List"])).await.json();
        assert_eq!(network_names(&list), ["streams"]);
        run(&import).await.fails(1, "taken");
    });
}

#[test]
fn malformed_calls_exit_2_and_say_why() {
    local(async {
        let s = Scratch::new("usage");
        let (db, pw) = (s.path("strims.redb"), s.path("pw"));
        let login = ["--profile", "alice", "--profile-password-file", pw.as_str()];
        let call = |method: &str, json: &str| {
            let mut v = vec!["call", "--db", db.as_str()];
            v.extend(login);
            v.extend([method, json]);
            v.into_iter().map(str::to_owned).collect::<Vec<_>>()
        };
        for (args, says) in [
            (call("NoSuch.Method", "{}"), "no frontend method"),
            (call("CreateServer", "{}"), "NetworkFrontend.CreateServer"),
            (
                call("NetworkFrontend.CreateServer", r#"{"nme": 1}"#),
                "CreateServerRequest",
            ),
            (
                call("NetworkFrontend.CreateServer", "{"),
                "CreateServerRequest",
            ),
            (
                vec![
                    "call".into(),
                    "--db".into(),
                    db.clone(),
                    "NetworkFrontend.List".into(),
                ],
                "needs a profile",
            ),
            (vec!["call".into(), "--nope".into()], "unknown argument"),
        ] {
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            run(&args).await.fails(2, says);
        }
        // nothing reached the store
        assert!(!Path::new(&db).exists());

        let methods = run(&["methods", "network"]).await.ok();
        assert!(
            methods.contains("NetworkFrontend.CreateServer(CreateServerRequest)"),
            "{methods}"
        );
        assert!(!methods.contains("VideoChannel"), "{methods}");
        let described = run(&["describe", "NetworkFrontend.CreateServer"])
            .await
            .ok();
        assert!(described.contains("  string name = 1;"), "{described}");
        assert!(
            run(&["call", "--help"])
                .await
                .ok()
                .contains("strims-svc profiles")
        );
    });
}
