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
ScriptDirectory: /mnt/
ControllerUser: "root"
PublicControllerAddress: "51.51.51.2"
InterfaceConfig:
  PrivateKey: "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX="
  Address: "10.0.0.1/24"
  ListenPort: 51820
ScriptDirectory: /
```

```
lxc file push scripts/* strims-k8s/mnt/
```
