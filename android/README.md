install gomobile

```
go get golang.org/x/mobile/cmd/gomobile
gomobile init
```

build bridge library

```
mkdir app/libs
gomobile bind -o app/libs/bridge.aar -target=android -androidapi 21 ./bridge
```
