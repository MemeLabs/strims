# Device lab

A page that real devices on the LAN keep open, and a loopback controller
that runs code on them: for playback, capture and HLS checks on an iPhone,
or on Chrome and Firefox on another machine.

```
hack/lab/mkcert.sh 10.0.0.134          # once: the lab CA and the server's certificate
node hack/lab/server.mjs 10.0.0.134    # the dev machine's LAN address
```

- http://10.0.0.134:8080/ serves the lab CA. Install and trust it once on
  each device; on iOS, install the profile, then turn on full trust under
  Settings > General > About > Certificate Trust Settings. The CA is
  name-constrained to that one address, so its key vouches for no other
  host. `mkcert.sh` keeps an existing CA and makes only a new server
  certificate, so devices keep trusting it.
- https://10.0.0.134:8443/ is the agent page. Tap "Unlock" (it starts
  `window.labAudio`, and plays `window.labSpareVideo` unmuted so probes may
  play sound later without a tap) and "Tap to keep the screen awake". The
  page never reloads itself: a reload loses the tap.
- http://127.0.0.1:8090/ is the controller, on loopback only:
  `GET /clients`, `POST /eval?client=ID&timeout=MS` (the body runs as an
  async function body on the device, and its JSON-safe value comes back),
  `GET /log?n=N`.

A strims-svc on the LAN serves the UI's API and its node's links over
`wss://` with the lab's certificate, so the devices' browsers link to it
directly:

```
strims-svc --db ./strims.redb --api-listen 0.0.0.0:8083 --api-path /manage \
  --api-tls --ws-listen 0.0.0.0:8082 --ws-tls \
  --tls-cert hack/lab/tls/server.crt --tls-key hack/lab/tls/server.key
```

Signing in logs the node's bootstrap URL, `node started ...
ws_uri="wss://10.0.0.134:8082/<profile key>"`. Native nodes bootstrapping to
it pass `--tls-ca hack/lab/tls/ca.crt`.

`tls/` and the server's `events.jsonl` stay out of git (`.gitignore`).
