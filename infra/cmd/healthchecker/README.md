`config.yaml`
```yaml
healthCheckers:
  - 127.0.0.1:50051
  - 127.0.0.1:50052
  - 127.0.0.1:50053
cloudflare:
  token: xxxxxxxxxxxxxxxxxxxx-xxxxxxxxxxxxxxxxxxx
  zoneID: xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
  domain: kube-api.strims.gg
loadBalancers:
  # ie haproxy monitor-uri
  - http://10.0.0.1/status
  - http://10.0.0.2/status
  - http://10.0.0.3/status
checkInterval: 1s
```
