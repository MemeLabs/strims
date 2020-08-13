### notes

#### dashboard login duration

fix dashboard login ttl by adding `'--token-ttl=0'` to args in the kubernetes-dashboard container template

#### monitoring
in `infra/hack/kubernetes/prometheus`
```
go get -u github.com/jsonnet-bundler/jsonnet-bundler/cmd/jb
go get github.com/google/go-jsonnet/cmd/jsonnet
go get github.com/brancz/gojsontoyaml

jb init
./build.sh
kubectl create -f manifests/setup
kubectl create -f manifests
```
