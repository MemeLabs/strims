`infra.yaml`
```yaml
DB:
  Name: ""
  User: ""
  Pass: ""
  Host: ""
  Port: 5432
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
PublicControllerAddress: "51.51.51.2:51820"
InterfaceConfig:
  PrivateKey: "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX="
  Address: "10.0.0.1/24"
  ListenPort: 51820
```

### Push scripts into desired location
```
lxc file push scripts/* strims-k8s/mnt/
```

### Port forwarding WireGuard to LXC
```
lxc config device add strims-k8s proxy listen=udp:0.0.0.0:51820 connect=udp:127.0.0.1:51820
```
