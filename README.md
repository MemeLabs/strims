<br />
<div align="center">
  <img src=".github/gopher.png" align="right"
       alt="A cute gopher eating popcorn" width="200" height="275">
  <h1>strims</h1>
  <small>distributed live streaming</small>
</div>
<br />

[setup](#setup)

[web ui](#web-ui)

[headless service](#headless-service)

[streaming](#streaming)

[connecting remote peers](#connecting-remote-peers)

[checks](#checks)

The node is written in Rust (`crates/`). It runs natively as `strims-svc` and
in the browser as `strims-wasm`. `crates/PLAN.md` lists the remaining work and
`crates/CONVENTIONS.md` the rules for working on it. `infra/` holds the
deployment tooling, which is written in Go.

## setup

1. Install rustup. `rust-toolchain.toml` picks the toolchain, its components
   and the `wasm32-unknown-unknown` target.

1. Install `wasm-bindgen-cli` at strims-wasm's `wasm-bindgen` pin:

   ```sh
   $ cargo install wasm-bindgen-cli --version 0.2.129 --locked
   ```

1. Install node.js and the ui's dependencies:

   ```sh
   $ npm ci
   ```

1. Generate a self signed root certificate using the instructions in
   [/hack/tls](/hack/tls)

## web ui

1. Run the webpack dev server. Its strims-wasm loader builds the browser node
   with cargo and wasm-bindgen.

   ```sh
   $ npm run dev:web
   ```

   > _on Windows depending on the directory permissions this may throw an error, if
   this happens execute the command from the error message then retry the run command._

   ```
   Error: Command failed: git rev-parse HEAD ...
   ```

   ```sh
   $ git config --global --add safe.directory /workspaces/strims
   ```

1. In the web ui at `https://<dev machine IP>:8080` click `new login` and sign
   up. The profile's node runs in a worker in the page.

## headless service

1. Run `strims-svc`. It keeps its profiles in one redb file and runs one
   profile's node at a time. `--help` lists its options.

   ```sh
   $ cargo run --release -p strims-svc -- --db ./strims.redb \
       --api-path /manage --ws-listen 127.0.0.1:8082 --allow-insecure-ws
   ```

   The webpack dev server proxies `wss://<dev machine IP>:8080/manage` to the
   service's API on port 8083, and `wss://<dev machine IP>:8080/<hex key>` to
   its node's WebSocket listener on port 8082.

1. In the web ui click `new login`, toggle the `advanced` options, enter
   `wss://<dev machine IP>:8080/manage` and sign up. The profile's node runs in
   the service.

## streaming

1. Create a network

1. In video ingress, toggle `enable` and click `save changes`

1. Click `channels` at the bottom of the ingress form

1. Enter a title for your stream, select your network from the dropdown, and
   click `create channel`.

   > _on Windows, the RTMP address for this channel ingress should be `rtmp://0.0.0.0:1935/live`_

1. In the channels table click the menu icon to the right of the ingress url and
   click `copy stream key`

1. In OBS stream settings, choose `Custom...` from the service dropdown. Enter
  `rtmp://0.0.0.0:1935/live` for the server and paste the stream key you copied
  from the channel list.

   > _on Windows, this address should be `rtmp://127.0.0.1:1935/live`_

1. In OBS output settings set the keyframe interval to 1 and the bitrate to
   6000kbps

1. Close OBS settings and click `start streaming`

1. Click the strims icon to exit settings and click the network gem in the left
   nav. click the stream thumbnail in the network directory to watch the stream.

This stream is looped back through the headless service's frontend api. Continue
to connect a remote peer and watch the stream via p2p.

## connecting remote peers

1. In the web ui while logged into your headless service open the networks
   list in settings.

1. In the networks table click the menu icon to the right of the certificate
   expiry and choose `create invite`.

1. In the invite form click `create invite`

1. Create or log into a browser profile in the web ui.

1. From the `add network` menu in the networks list in settings choose `add
   invite code`, paste the invite code, and click `join network`

1. In bootstrap settings create a bootstrap client for the headless service's
   node, `wss://<dev machine IP>:8080/<hex key>`, where `<hex key>` is the
   headless profile's public key in hex

1. Find the stream thumbnail in the network directory

## checks

`crates/CONVENTIONS.md` lists the checks to run from the repo root before
committing: formatting, clippy, the workspace's tests, the wasm build, and
strims-wasm's browser suites.
