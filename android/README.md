install gomobile

```
go get golang.org/x/mobile/cmd/gomobile
gomobile init
```

build bridge library

```
gomobile bind -o app/bridge.aar -target=android -androidapi 21 ./bridge
```
