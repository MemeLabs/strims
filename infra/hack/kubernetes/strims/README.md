```
kubectl apply -f https://raw.githubusercontent.com/rancher/local-path-provisioner/master/deploy/local-path-storage.yaml
```

```
curl https://raw.githubusercontent.com/helm/helm/master/scripts/get-helm-3 | bash
```

```
helm repo add bitnami https://charts.bitnami.com/bitnami
helm install postgres bitnami/postgresql -n strims -f helm/postgres/values.yaml
helm install svc-db bitnami/postgresql -n strims -f helm/postgres/values.yaml
```

```
helm repo add nginx-stable https://helm.nginx.com/stable
helm install nginx nginx-stable/nginx-ingress -n strims
```
