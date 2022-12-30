# test/e2e

E2E tests can be executed against a
[KiND](https://github.com/kubernetes-sigs/kind) cluster or an existing cluster
by providing the proper flags like so:

```
❯ go test -v ./test/e2e/ -run TestSimpleStream -args -kubeconfig <path to kubeconfig> -strims.use-existing-deployment -strims.namespace <namespace>
```

Along with `go test` flags, and the `kubernetes-sigs/e2e-framework`
[flags](https://github.com/kubernetes-sigs/e2e-framework/tree/main/examples/flags#supported-flags),
a custom set of flags is defined to control test behavior:

```
❯ go test -v ./test/e2e/ -run TestSimpleStream -args -fail-fast -help
...
  -strims.controller-ip string
    	IP of the node exposing svc (default "10.0.0.1")
  -strims.debug-port int
    	svc debug port (default 30001)
  -strims.image string
    	svc container image
  -strims.invites-port int
    	svc invites port (default 30005)
  -strims.metrics-port int
    	svc metrics port (default 30000)
  -strims.namespace string
    	test namespace to use
  -strims.rtmp-port int
    	svc RTMP port (default 1935)
  -strims.use-existing-deployment
    	utilize an existing deployment of svc
  -strims.webrtc-port int
    	svc webrtc port (default 30003)
  -strims.ws-port int
    	svc websocket port (default 30002)
...
```
