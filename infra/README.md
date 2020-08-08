`infra.yaml`
```yaml
DB:
  Path: ./hack/dev.sqlite
FlakeStartTime: 2019-09-14T00:00:00Z
Providers:
  digitalocean:
    driver: DigitalOcean
    Token: xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
  scaleway:
    driver: Scaleway
    OrganizationID: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
    AccessKey: xxxxxxxxxxxxxxxxxxxx
    SecretKey: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
  hetzner:
    driver: Hetzner
    Token: xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
SSH:
  IdentityFile: /root/.ssh/id_ecdsa_example
```
