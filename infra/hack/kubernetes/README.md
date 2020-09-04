I wasn't really sure why you used ### notes

#### deploying
in `infra/hack/kubernetes/monitoring` run
```
./deploy-monitoring.sh
```

#### accessing services
retrieve grafana admin username and password
```
echo "Admin User:" $(kubectl get secret -n monitoring grafana-admin -o jsonpath="{.data.admin-user}" | base64 --decode) && \
echo "Admin Password:" $(kubectl get secret -n monitoring grafana-admin -o jsonpath="{.data.admin-password}" | base64 --decode)
```

add an entry to /etc/hosts
```
<node ip> strims.monitoring.local 
```
replacing node ip with the ip address of the node that's hosting the ingress

```
access grafana at http://strims.monitoring.local/grafana/
```
```
access prometheus at http://strims.monitoring.local/prometheus
```

#### dashboard login duration

fix dashboard login ttl by adding `'--token-ttl=0'` to args in the kubernetes-dashboard container template

#### dashboards
##### kubernetes
```
kube proxy
```
http://localhost:8001/api/v1/namespaces/kubernetes-dashboard/services/https:kubernetes-dashboard:/proxy/


