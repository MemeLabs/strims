## Style Guide
Replace any variables enclosed in <variables>

For example: helm install nginx -n \<my namespace\> becomes helm install nginx -n develop

## Installing coturn
```
helm dependency update coturn/ && helm install coturn -n <my namespace> -f coturn/values.yaml coturn/
```
