# strims

distributed live streaming

[headless service](#headless-service)

[streaming](#streaming)

[connecting remote peers](#connecting-remote-peers)

## headless service

1. Generate a self signed root certificate using the instructions in
   [/hack/tls](/hack/tls)

1. Copy `hack/svc/config.sample.yaml` to `hack/svc/config.yaml`

1. Create a user profile

   ```sh
   $ cd /workspaces/strims
   $ go run ./cmd/svc add-profile -config ./hack/svc/config.yaml -username dev -password secret
   2022/06/01 13:48:18 74880235203461122
   2022/06/01 13:48:18 DnLta2pK2GlRLHTobM2jKecX/XNqD4OGDjwWSOs0o7s=
   ```

1. Copy the id and auth key into `config.yaml` in `session/headless` like so:

   ```yaml
   headless:
      - id: 74880235203461122
        key: DnLta2pK2GlRLHTobM2jKecX/XNqD4OGDjwWSOs0o7s=
   ```

1. Run `svc` with live reloading using npm

   ```sh
   $ npm run dev:svc
   ```

   _Take note of the path from the line `ws vnic listener starting` - you'll need it
   to connect to the headless service from the ui._

   ```
   2022-06-01T13:54:33.545Z        DEBUG   vnic/ws_native.go:43    ws vnic listener starting       {"path": "/0408549bb2971e6051c11c1caafbbd2b0f88b6937d0bf608f6d1a2d4c237a0f8"}
   ```

1. Run the webpack dev server

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

1. In the web ui at `https://0.0.0.0:8080` click `new login` and enter the
   username and password you used in the `add-profile` command.

1. Toggle the advanced `advanced` options and enter `wss://0.0.0.0:8083/api` or
   the IP of your dev machine or localhost.

   > _on Windows, this address might have to be set as be `wss://localhost:8083/api`._

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

1. In the web ui while logged into your headless server open the networks list
   in settings.

1. In the networks table click the menu icon to the right of the certificate
   expiry and choose `create invite`.

1. In the invite form click `create invite`

1. Create or log into a browser profile in the web ui.

1. From the `add network` menu in the networks list in settings choose `add
   invite code`, paste the invite code, and click `join network`

1. In bootstrap settings create a bootstrap client using the path from the
   headless service log earlier eg
   `wss://0.0.0.0:8083/0408549bb2971e6051c11c1caafbbd2b0f88b6937d0bf608f6d1a2d4c237a0f8`

1. Find the stream thumbnail in the network directory
