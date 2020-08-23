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

#### dashboards
##### kubernetes
```
kube proxy
```
http://localhost:8001/api/v1/namespaces/kubernetes-dashboard/services/https:kubernetes-dashboard:/proxy/
##### grafana
```
kubectl --namespace monitoring port-forward svc/grafana 3000
```
##### prometheus
```
kubectl --namespace monitoring port-forward svc/prometheus-k8s 9090
```
