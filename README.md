strims

distributed live streaming

## headless service

1.) generate a self signed root certificate using the instructions in `hack/tls`

2.) copy `hack/svc/config.sample.yaml` to `hack/svc/config.yaml`

3.) create a user profile
```sh
$ go run ./cmd/svc add-profile -config ./hack/svc/config.yaml -username dev -password secret
2022/06/01 13:48:18 74880235203461122
2022/06/01 13:48:18 DnLta2pK2GlRLHTobM2jKecX/XNqD4OGDjwWSOs0o7s=
```

4.) copy the id and auth key into `config.yaml` in `session/headless` ex:
```yaml
  headless:
    - id: 74880235203461122
      key: DnLta2pK2GlRLHTobM2jKecX/XNqD4OGDjwWSOs0o7s=
```

5.) run `svc` with live reloading using npm
```
$ npm run dev:svc
```

take note of the path from the line `ws vnic listener starting` - you'll need it to connect to the headless service from the ui.
```
2022-06-01T13:54:33.545Z        DEBUG   vnic/ws_native.go:43    ws vnic listener starting       {"path": "/0408549bb2971e6051c11c1caafbbd2b0f88b6937d0bf608f6d1a2d4c237a0f8"}
```

6.) run the webpack dev server
```
npm run dev:web
```

on windows depending on the directory permissions this may throw an error
```
Error: Command failed: git rev-parse HEAD ...
```
if this happens run the command from the error message then retry the run command.
```
git config --global --add safe.directory /workspaces/strims
```

7.) in the web ui at `https://0.0.0.0:8080` click `new login` and enter the username and password you used in the `add-profile` command.

8.) toggle the advanced `advanced` options and enter `wss://0.0.0.0:8083/api` or the ip of your dev machine or localhost.

## streaming

1.) create a network

2.) in video ingress toggle `enable` and click `save changes`

3.) click `channels` at the bottom of the ingress form

4.) enter a title for your stream, select your network from the dropdown, and click `create channel`

5.) in the channels table click the menu icon to the right of the ingress url and click `copy stream key`

6.) in obs stream settings choose `Custom...` from the service dropdown. enter `rtmp://0.0.0.0:1935/live` for the server and paste the stream key you copied from the channel list.

7.) in obs output settings set the keyframe interval to 1 and the bitrate to 6000kbps

8.) close obs settings and click `start streaming`

9.) click the strims icon to exit settings and click the network gem in the left nav. click the stream thumbnail in the network directory to watch the stream.

this stream is looped back through the headless service's frontend api. continue to connect a remote peer and watch the stream via p2p.

## connecting remote peers

1.) in the web ui while logged into your headless server open the networks list in settings.

2.) in the networks table click the menu icon to the right of the certificate expiry and choose `create invite`.

3.) in the invite form click `create invite`

4.) create or log into a browser profile in the web ui.

5.) from the `add network` menu in the networks list in settings choose `add invite code`, paste the invite code, and click `join network`

5.) in bootstrap settings create a bootstrap client using the path from the headless service log earlier eg `wss://0.0.0.0:8083/0408549bb2971e6051c11c1caafbbd2b0f88b6937d0bf608f6d1a2d4c237a0f8`

6.) find the stream thumbnail in the network directory
