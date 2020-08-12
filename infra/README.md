`infra.yaml`
```yaml
DB:
  Path: ./hack/dev.sqlite
FlakeStartTime: 2019-09-14T00:00:00Z
Providers:
  digitalocean:
    Driver: DigitalOcean
    Token: xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
  scaleway:
    Driver: Scaleway
    OrganizationID: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
    AccessKey: xxxxxxxxxxxxxxxxxxxx
    SecretKey: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
  hetzner:
    Driver: Hetzner
    Token: xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
  ovh:
    Driver: OVH
    AppKey: xxxxxxxxxxxxxxxx
    AppSecret: xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
    ConsumerKey: xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
    ProjectID: xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
    Subsidiary: xx
  dreamhost:
    Driver: DreamHost
    TenantID: xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
    TenantName: xxxxxxxxxx
    Username: xxxxxxxx
    Password: xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
  heficed:
    Driver: Heficed
    ClientID: xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
    ClientSecret: xxxxxxxxxxxx
    TenantID: xxxxxx-xxxx-xxxx-xxxx-xxxxxxxxx
SSH:
  IdentityFile: /root/.ssh/id_ecdsa_example
```
