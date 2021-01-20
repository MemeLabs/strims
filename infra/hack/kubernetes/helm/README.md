## Style Guide
Replace any variables enclosed in `<variable>`

For example: `helm install nginx -n <my namespace>` becomes `helm install nginx -n develop`

## Installing coturn
```
helm dependency update coturn/ && helm install coturn -n <my namespace> -f coturn/values.yaml coturn/
```

## Postgres
```
helm repo add bitnami https://charts.bitnami.com/bitnami
helm install strims-infra-db -f go-ppspp/infra/hack/kubernetes/helm/postgres/values.yaml bitnami/postgresql
```

## Master has a NoSchedule taint
To remove it run:
```
kubectl taint nodes controller node-role.kubernetes.io/master:NoSchedule-
```
