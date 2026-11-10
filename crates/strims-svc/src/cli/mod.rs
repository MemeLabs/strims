// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! `strims-svc`'s command line beside running a node ([`USAGE`]): calls to
//! the frontend API the UI calls, every method of every service it serves
//! read from the schema's descriptors, and a store's profiles.
//!
//! A call goes over a running service's API, or one the command line runs
//! on a store for the call; it signs in as a profile first unless the
//! method is the auth service's. Requests and responses are proto3 JSON:
//! the schema's field names, `bytes` in base64, 64-bit integers as strings,
//! enums by name.

mod connect;
mod profiles;
mod schema;

use std::io::{Read as _, Write as _};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration as StdDuration;

use futures_util::StreamExt;
use tokio::task::LocalSet;
use tokio::time::{Instant, sleep_until};

pub use connect::{DEFAULT_API, Target};
pub use profiles::ProfilesCommand;

use connect::Connection;
use schema::{AUTH_SERVICE, Schema, Style, short_name};

/// The subcommands' usage.
pub const USAGE: &str = "\
usage: strims-svc call <Service.Method> [<json> | -] [options]
       strims-svc methods [<filter>]
       strims-svc describe <Service.Method | message | enum>
       strims-svc decode <message> [<base64> | -]
       strims-svc encode <message> [<json> | -]
       strims-svc profiles (list | add | remove | export | import) --db <path> [options]

Calls the frontend API the UI calls, every operation of its pages and
settings, and manages a store's profiles.

call: calls a frontend method and prints its response as JSON, or a
streaming method's responses one JSON line each. The request is JSON
(default {}), or - to read it from stdin. JSON is proto3's mapping: the
schema's field names (camelCase too on input), bytes in base64, 64-bit
integers as strings (numbers too on input), enums by name.
  --api <url>               a running service's API (default
                            ws://127.0.0.1:8083/api: its --api-listen and
                            --api-path)
  --ca <path>               a PEM file of roots a wss:// API's certificate
                            may chain to, besides the public ones
  --db <path>               with no service running on this store: run one
                            for the call, the profile's node too
  --profile <name>          the profile to sign in as, with
  --profile-password-file <path>
                            the file holding its password; AuthFrontend's
                            methods need neither
  --limit <n>               end a streaming call after n responses
  --timeout <secs>          end a streaming call after secs (default:
                            when it ends, or at ctrl-c); fail a unary call
                            after secs (default 30)
  --defaults                print fields at their default values too

methods: the frontend methods, with their request and response types;
those whose Service.Method contains <filter>, in any case.

describe: a method's request and response messages, or a message or an
enum, as the schema declares them, comments included.

decode, encode: a message carried encoded in a bytes field, base64 to JSON
and back: a signed envelope's body (a strims.type.Certificate's is a
strims.type.CertificateBody; a network's key is its root certificate's
body's key), an invitation's data (strims.network.v1.InvitationV0).

profiles: a store's profiles, while no service runs on it.
  list                      its profiles' names, one a line
  add                       signs --profile up with the password in
                            --profile-password-file
  remove                    deletes --profile and its store; needs its
                            password
  export --out <path>       writes --profile to a new file: its sign-in
                            record and store, still sealed under its
                            password and key, so as sensitive as the store
  import --in <path>        adds an exported profile, and prints its name
An export is a backup or a move: an import running beside its original is
the same device. A second device pairs: ReplicationFrontend.
CreatePairingToken, then AuthFrontend.SignIn with the token where the new
device's store is.

A usage error (an unknown method, a request that isn't its type) exits 2;
a failure (connecting, signing in, an RPC error) exits 1.

examples:
  strims-svc profiles add --db strims.redb --profile alice --profile-password-file pw
  strims-svc call --db strims.redb --profile alice --profile-password-file pw \\
      NetworkFrontend.CreateServer '{\"name\": \"streams\"}'
  strims-svc describe VideoChannelFrontend.Create
  strims-svc decode strims.type.CertificateBody <a certificate's body>
  strims-svc call --profile alice --profile-password-file pw --limit 1 \\
      DirectoryFrontend.WatchListings '{\"content_types\": [\"LISTING_CONTENT_TYPE_MEDIA\"]}'";

/// A profile's name and password, which the command line signs in with.
#[derive(Clone)]
pub struct ProfileLogin {
    pub name: String,
    pub password: String,
}

impl std::fmt::Debug for ProfileLogin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProfileLogin")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

/// The longest a unary call waits without `--timeout`.
const UNARY_TIMEOUT: StdDuration = StdDuration::from_secs(30);

/// What the command line asks for beside running a node.
#[derive(Debug)]
pub enum Command {
    /// Prints [`USAGE`].
    Help,
    Call(CallArgs),
    /// Lists the frontend methods whose `Service.Method` contains the
    /// filter.
    Methods {
        filter: Option<String>,
    },
    /// Describes a method, message or enum.
    Describe {
        name: String,
    },
    /// Prints an encoded message, base64 or `-` for stdin, as JSON.
    Decode {
        message: String,
        input: Option<String>,
    },
    /// Prints a message given as JSON, or `-` for stdin, encoded, in
    /// base64.
    Encode {
        message: String,
        input: Option<String>,
    },
    Profiles(ProfilesCommand),
}

/// A frontend call.
#[derive(Debug)]
pub struct CallArgs {
    /// `Service.Method`, the package optional, in any case.
    pub method: String,
    /// The request as JSON, `-` for stdin, or `None` for the empty request.
    pub request: Option<String>,
    pub target: Target,
    /// The profile to sign in as first.
    pub login: Option<ProfileLogin>,
    /// The most responses a streaming call prints.
    pub limit: Option<usize>,
    pub timeout: Option<StdDuration>,
    /// Prints fields at their default values too.
    pub defaults: bool,
}

/// Whether `arg`, the first argument, names a subcommand.
pub fn is_subcommand(arg: &str) -> bool {
    matches!(
        arg,
        "call" | "methods" | "describe" | "decode" | "encode" | "profiles"
    )
}

/// Parses the arguments after subcommand `command`. Flags take their
/// value as the next argument or after `=`.
pub fn parse(command: &str, args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let mut positional = Vec::new();
    let mut api = None;
    let mut ca = None;
    let mut db = None;
    let mut profile = None;
    let mut password_file = None;
    let mut limit = None;
    let mut timeout = None;
    let mut defaults = false;
    let mut out = None;
    let mut input = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "-h" {
            return Ok(Command::Help);
        }
        if !arg.starts_with("--") {
            positional.push(arg);
            continue;
        }
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
        match flag.as_str() {
            "--help" => return Ok(Command::Help),
            "--api" => api = Some(value()?),
            "--ca" => ca = Some(PathBuf::from(value()?)),
            "--db" => db = Some(PathBuf::from(value()?)),
            "--profile" => profile = Some(value()?),
            "--profile-password-file" => password_file = Some(PathBuf::from(value()?)),
            "--limit" => {
                let v = value()?;
                limit = Some(
                    v.parse::<usize>()
                        .ok()
                        .filter(|n| *n > 0)
                        .ok_or_else(|| format!("--limit takes a count of 1 or more: {v}"))?,
                );
            }
            "--timeout" => {
                let v = value()?;
                let secs = v
                    .parse::<f64>()
                    .ok()
                    .filter(|s| s.is_finite() && *s > 0.0)
                    .ok_or_else(|| format!("--timeout takes seconds over 0: {v}"))?;
                timeout = Some(StdDuration::from_secs_f64(secs));
            }
            "--defaults" => defaults = true,
            "--out" => out = Some(PathBuf::from(value()?)),
            "--in" => input = Some(PathBuf::from(value()?)),
            _ => return Err(format!("unknown argument {flag}")),
        }
    }
    let login = match (profile, password_file) {
        (Some(name), Some(path)) => Some(ProfileLogin {
            name,
            password: read_password_file(&path)?,
        }),
        (None, None) => None,
        _ => return Err("--profile and --profile-password-file go together".to_owned()),
    };
    let mut positional = positional.into_iter();
    let command = match command {
        "call" => {
            let method = positional
                .next()
                .ok_or("call needs a method: strims-svc methods lists them")?;
            let request = positional.next();
            let target = match (api, db) {
                (Some(_), Some(_)) => return Err("pass --api or --db, not both".to_owned()),
                (_, Some(db)) if ca.is_some() => {
                    return Err(format!("--ca goes with --api, not --db {}", db.display()));
                }
                (_, Some(db)) => Target::Db(db),
                (url, None) => Target::Api {
                    url: url.unwrap_or_else(|| DEFAULT_API.to_owned()),
                    ca,
                },
            };
            Command::Call(CallArgs {
                method,
                request,
                target,
                login,
                limit,
                timeout,
                defaults,
            })
        }
        "methods" => Command::Methods {
            filter: positional.next(),
        },
        "describe" => Command::Describe {
            name: positional
                .next()
                .ok_or("describe needs a method, message or enum")?,
        },
        "decode" => Command::Decode {
            message: positional.next().ok_or("decode needs a message type")?,
            input: positional.next(),
        },
        "encode" => Command::Encode {
            message: positional.next().ok_or("encode needs a message type")?,
            input: positional.next(),
        },
        "profiles" => {
            let action = positional
                .next()
                .ok_or("profiles needs list, add, remove, export or import")?;
            let db = db.ok_or("profiles needs --db <path>")?;
            let login = || {
                login.clone().ok_or(format!(
                    "profiles {action} needs --profile and --profile-password-file"
                ))
            };
            Command::Profiles(match action.as_str() {
                "list" => ProfilesCommand::List { db },
                "add" => ProfilesCommand::Add {
                    db,
                    login: login()?,
                },
                "remove" => ProfilesCommand::Remove {
                    db,
                    login: login()?,
                },
                "export" => ProfilesCommand::Export {
                    db,
                    login: login()?,
                    out: out.ok_or("profiles export needs --out <path>")?,
                },
                "import" => ProfilesCommand::Import {
                    db,
                    input: input.ok_or("profiles import needs --in <path>")?,
                },
                a => {
                    return Err(format!(
                        "profiles takes list, add, remove, export or import: {a}"
                    ));
                }
            })
        }
        c => return Err(format!("unknown command {c}")),
    };
    if let Some(extra) = positional.next() {
        return Err(format!("unexpected argument {extra}"));
    }
    Ok(command)
}

/// The password in the file at `path`, without one trailing newline.
pub fn read_password_file(path: &std::path::Path) -> Result<String, String> {
    let mut s = std::fs::read_to_string(path)
        .map_err(|e| format!("--profile-password-file {}: {e}", path.display()))?;
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
    Ok(s)
}

/// Why a command didn't do what it was asked.
#[derive(Debug)]
pub(crate) enum Failure {
    /// The command was malformed: exit 2.
    Usage(String),
    /// It failed: exit 1.
    Failed(String),
    /// Whatever reads stdout closed it, so the command stops, quietly.
    StdoutClosed,
}

/// Runs `command` on a runtime of its own; logs go to stderr.
pub fn run(command: Command) -> ExitCode {
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("strims-svc: starting the runtime: {e}");
            return ExitCode::FAILURE;
        }
    };
    match rt.block_on(LocalSet::new().run_until(dispatch(command))) {
        Ok(()) | Err(Failure::StdoutClosed) => ExitCode::SUCCESS,
        Err(Failure::Usage(e)) => {
            eprintln!("strims-svc: {e}");
            ExitCode::from(2)
        }
        Err(Failure::Failed(e)) => {
            eprintln!("strims-svc: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn dispatch(command: Command) -> Result<(), Failure> {
    match command {
        Command::Help => stdout_line(USAGE),
        Command::Methods { filter } => {
            let list = Schema::load().list(filter.as_deref());
            stdout_line(list.trim_end())
        }
        Command::Describe { name } => {
            let d = Schema::load().describe(&name).map_err(Failure::Usage)?;
            stdout_line(d.trim_end())
        }
        Command::Decode { message, input } => {
            let schema = Schema::load();
            let desc = schema.find_message(&message).map_err(Failure::Usage)?;
            let text = read_input(input.as_deref())?;
            let bytes = base64(text.trim()).map_err(Failure::Usage)?;
            let style = Style {
                pretty: true,
                defaults: false,
            };
            stdout_line(&schema::response(&desc, &bytes, style).map_err(Failure::Usage)?)
        }
        Command::Encode { message, input } => {
            let schema = Schema::load();
            let desc = schema.find_message(&message).map_err(Failure::Usage)?;
            let json = read_input(input.as_deref())?;
            let bytes = schema::encode(&desc, &json)
                .map_err(|e| Failure::Usage(format!("the JSON {e}")))?;
            stdout_line(&data_encoding::BASE64.encode(&bytes))
        }
        Command::Profiles(p) => profiles::run(p).await,
        Command::Call(args) => call(args).await,
    }
}

/// The argument `input`, stdin for `-`, or nothing.
fn read_input(input: Option<&str>) -> Result<String, Failure> {
    match input {
        None => Ok(String::new()),
        Some("-") => {
            let mut s = String::new();
            std::io::stdin()
                .read_to_string(&mut s)
                .map_err(|e| Failure::Failed(format!("reading stdin: {e}")))?;
            Ok(s)
        }
        Some(s) => Ok(s.to_owned()),
    }
}

/// `s` in base64, standard or URL-safe, padded or not.
fn base64(s: &str) -> Result<Vec<u8>, String> {
    use data_encoding::{BASE64, BASE64_NOPAD, BASE64URL, BASE64URL_NOPAD};
    [BASE64, BASE64_NOPAD, BASE64URL, BASE64URL_NOPAD]
        .iter()
        .find_map(|e| e.decode(s.as_bytes()).ok())
        .ok_or_else(|| format!("not base64: {s}"))
}

async fn call(args: CallArgs) -> Result<(), Failure> {
    let schema = Schema::load();
    let method = schema.method(&args.method).map_err(Failure::Usage)?;
    let name = short_name(&method);
    let json = read_input(args.request.as_deref())?;
    let req = schema::request(&method, &json).map_err(Failure::Usage)?;
    let auth = method.parent_service().full_name() == AUTH_SERVICE;
    if args.login.is_none() && !auth {
        return Err(Failure::Usage(format!(
            "{name} needs a profile: pass --profile and --profile-password-file"
        )));
    }
    let conn = Connection::open(&args.target)
        .await
        .map_err(Failure::Failed)?;
    let done = async {
        if let Some(login) = args.login.as_ref().filter(|_| !auth) {
            conn.sign_in(login).await.map_err(Failure::Failed)?;
        }
        let failed = |e: &dyn std::fmt::Display| Failure::Failed(format!("{name}: {e}"));
        let output = method.output();
        if !method.is_server_streaming() {
            let call = conn.client.call_encoded(method.full_name(), &req);
            let res = tokio::time::timeout(args.timeout.unwrap_or(UNARY_TIMEOUT), call)
                .await
                .map_err(|_| failed(&"timed out"))?
                .map_err(|e| failed(&e))?;
            let style = Style {
                pretty: true,
                defaults: args.defaults,
            };
            let json = schema::response(&output, &res, style).map_err(Failure::Failed)?;
            return stdout_line(&json);
        }
        let mut stream = conn
            .client
            .call_streaming_encoded(method.full_name(), &req)
            .map_err(|e| failed(&e))?;
        let deadline = args.timeout.map(|t| Instant::now() + t);
        let style = Style {
            pretty: false,
            defaults: args.defaults,
        };
        let mut printed = 0;
        loop {
            let next = tokio::select! {
                biased;
                _ = tokio::signal::ctrl_c() => break,
                () = until(deadline) => break,
                next = stream.next() => next,
            };
            match next {
                None => break,
                Some(Ok(b)) => {
                    let json = schema::response(&output, &b, style).map_err(Failure::Failed)?;
                    stdout_line(&json)?;
                    printed += 1;
                    if args.limit.is_some_and(|l| printed >= l) {
                        break;
                    }
                }
                Some(Err(e)) => return Err(failed(&e)),
            }
        }
        Ok(())
    }
    .await;
    conn.close().await;
    done
}

/// Resolves at `deadline`, or never.
async fn until(deadline: Option<Instant>) {
    match deadline {
        Some(d) => sleep_until(d).await,
        None => std::future::pending().await,
    }
}

/// Writes `s` and a newline to stdout, at once.
pub(crate) fn stdout_line(s: &str) -> Result<(), Failure> {
    let mut out = std::io::stdout().lock();
    writeln!(out, "{s}")
        .and_then(|()| out.flush())
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::BrokenPipe => Failure::StdoutClosed,
            _ => Failure::Failed(format!("writing to stdout: {e}")),
        })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]

    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|a| (*a).to_owned()).collect()
    }

    #[test]
    fn calls_go_to_the_default_api_unless_given_a_store() {
        let Command::Call(c) = parse("call", args(&["NetworkFrontend.List"])).unwrap() else {
            panic!("not a call");
        };
        assert_eq!(
            c.target,
            Target::Api {
                url: DEFAULT_API.into(),
                ca: None
            }
        );
        assert_eq!((c.request, c.limit, c.defaults), (None, None, false));

        let Command::Call(c) = parse(
            "call",
            args(&[
                "--db=s.redb",
                "X.Y",
                "{}",
                "--limit",
                "2",
                "--timeout=1.5",
                "--defaults",
            ]),
        )
        .unwrap() else {
            panic!("not a call");
        };
        assert_eq!(c.target, Target::Db("s.redb".into()));
        assert_eq!(c.request.as_deref(), Some("{}"));
        assert_eq!(c.limit, Some(2));
        assert_eq!(c.timeout, Some(StdDuration::from_millis(1500)));
        assert!(c.defaults);
    }

    #[test]
    fn help_follows_any_subcommand() {
        for (command, list) in [
            ("call", &["--help"][..]),
            ("call", &["NetworkFrontend.List", "-h"][..]),
            ("profiles", &["-h"][..]),
            ("decode", &["--help"][..]),
        ] {
            assert!(
                matches!(parse(command, args(list)), Ok(Command::Help)),
                "{command} {list:?}"
            );
        }
    }

    #[test]
    fn malformed_commands_are_refused() {
        for (command, list) in [
            ("call", &[][..]),
            ("call", &["X.Y", "--api", "ws://a", "--db", "b"][..]),
            ("call", &["X.Y", "--db", "b", "--ca", "c"][..]),
            ("call", &["X.Y", "{}", "extra"][..]),
            ("call", &["X.Y", "--limit", "0"][..]),
            ("call", &["X.Y", "--timeout", "-1"][..]),
            ("call", &["X.Y", "--profile", "alice"][..]),
            ("describe", &[][..]),
            ("decode", &[][..]),
            ("encode", &["a", "b", "c"][..]),
            ("profiles", &["list"][..]),
            ("profiles", &["rename", "--db", "a"][..]),
            ("profiles", &["add", "--db", "a"][..]),
            ("profiles", &["import", "--db", "a"][..]),
            ("nope", &[][..]),
        ] {
            assert!(parse(command, args(list)).is_err(), "{command} {list:?}");
        }
    }

    #[test]
    fn profile_commands_read_the_password_file() {
        let dir = std::env::temp_dir().join(format!("strims-cli-args-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pw = dir.join("pw");
        std::fs::write(&pw, "s3cret\r\n").unwrap();
        let pw = pw.to_str().unwrap();
        let Command::Profiles(ProfilesCommand::Export { login, out, .. }) = parse(
            "profiles",
            args(&[
                "export",
                "--db",
                "s.redb",
                "--profile",
                "alice",
                "--profile-password-file",
                pw,
                "--out",
                "alice.export",
            ]),
        )
        .unwrap() else {
            panic!("not an export");
        };
        assert_eq!(
            (login.name.as_str(), login.password.as_str()),
            ("alice", "s3cret")
        );
        assert_eq!(out, PathBuf::from("alice.export"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
