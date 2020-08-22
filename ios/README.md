install gomobile
```
go get golang.org/x/mobile/cmd/gomobile
gomobile init
```

build bridge library
```
gomobile bind -target ios -o ./App/Bridge.framework ./bridge
```
