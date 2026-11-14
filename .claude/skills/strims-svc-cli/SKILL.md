---
name: strims-svc-cli
description: strims-svc's command line, which runs every UI operation from the shell as JSON (call, methods, describe, decode, encode, profiles), and the headless service's own flags, including a profile booted without a UI and the bootstrap API under /api/v1/ that the watch page reads. Use when calling the node's frontend RPCs from a shell or a script, seeding demo data, booting a headless profile, or changing or serving the watch page's bootstrap API.
---

# strims-svc from the shell

`strims-svc` (crates/strims-svc) runs a profile's node and serves the
UI's frontend RPC. Its subcommands call that same API from a shell, so
every operation of the UI's pages and settings works without a browser.
Seed demo data with them, not with throwaway test files or scripts. Run
it as `cargo run -q -p strims-svc -- ARGS`. `strims-svc --help` prints
the service's flags (`args::USAGE`), and `strims-svc call --help` prints
the subcommands' (`cli::USAGE`). The README's "command line" section
walks through creating a network, a channel and an ingress.

## The subcommands

    strims-svc methods [FILTER]                      # Service.Method, request and response types
    strims-svc describe Service.Method|MESSAGE|ENUM  # as schema/ declares it, comments included
    strims-svc call Service.Method ['JSON' | -] [--api URL | --db STORE] [--profile NAME --profile-password-file FILE]
    strims-svc decode|encode MESSAGE ['DATA' | -]    # a message carried in a bytes field, base64 to JSON and back
    strims-svc profiles list|add|remove|export|import --db STORE ...

- **JSON** follows proto3's mapping: the schema's field names, bytes in
  base64, 64-bit integers as strings, enums by name. `--defaults` prints
  fields at their default values too. `methods` and `describe` read the
  descriptors compiled into the binary, so they match its schema/.
- **Where a call goes.** `--api` names a running service, by default
  ws://127.0.0.1:8083/api (`--ca` adds roots for wss://). `--db` runs a
  service on a store for the length of the call, the profile's node
  included, and works only on an existing store that no service holds.
- **Signing in.** `--profile` with `--profile-password-file` signs in
  first. AuthFrontend's methods need neither.
- **Streams.** A streaming method prints one JSON line per response.
  `--limit N` and `--timeout SECS` end it. A unary call fails after 30 s.
- **Keys.** A network's key is its root certificate's body's key:
  `decode strims.type.CertificateBody` on the network's
  `certificate.parent.body`.
- **profiles** works on a store that no service or command holds. `add`
  creates the store if needed. `export --out` and `import --in` back up
  or move a profile, still sealed: an import running beside its original
  is the same device. A second device pairs instead
  (ReplicationFrontend.CreatePairingToken, then AuthFrontend.SignIn).
- A usage error exits 2, and a failure (connecting, signing in, an RPC
  error) exits 1.

## Headless profile and bootstrap API

`--profile NAME --profile-password-file FILE` on the service signs in at
start and runs that profile's node without a UI. The profile must exist,
and a failed sign-in fails the start. A UI that signs in later joins the
same session.

`--bootstrap-network KEY` (repeatable, the key in unpadded URL-safe
base64) with `--hls-listen` and `--bootstrap-ws-url` serves the bootstrap
API on the HLS listener (crates/strims-svc/src/bootstrap.rs). The watch
page (src/watch, `watch.html` at /watch) reads it to list a network's
streams and join from a vended invitation. The README's "one-click watch
page" section runs it end to end. crates/strims-svc/tests/bootstrap.rs
and crates/strims-wasm/tests/ui_watch.rs test it.

The code holds these rules. Changes must keep them:

- **Owner-only projection.** The API serves only networks whose CA the
  running profile holds. A start naming another network fails
  (`SvcError::NotOwner`), and a network whose CA the profile no longer
  holds answers 404 (`Bootstrap::owned`).
- **Vended invitations are the owner's own.** No vending identity issues
  them. The CA exempts the owner's invitations from the invite quota
  (crates/strims-network/src/ca.rs, `the_owners_invitations_spend_no_quota`),
  so one invitation admits any number of newcomers until it expires.
  Don't add a quota check or a separate identity for the owner.
- **One prefix.** Every route is under `/api/v1/` (`bootstrap::PREFIX`):
  `directory/events`, `directory/thumbnails/...` and `invitation/<network>`.
  Without the bootstrap config these paths answer 503. New routes go
  under the same prefix, and src/watch/api.ts names them for the page.
- Event streams are capped at half the listener's connections. Only
  raster thumbnails are served, with nosniff and a sandboxing CSP,
  because the API may share an origin with the UI's stored keys.
