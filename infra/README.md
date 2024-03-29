
### Config file

```yaml
LogLevel: -1
DB:
  Name: ""
  User: ""
  Pass: ""
  Host: ""
  Port: 5432
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
SSHIdentityFile: /root/.ssh/id_ecdsa_example
ScriptDirectory: ./hack/kubernetes
CertificateKey: xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
Lockfile: ./cli.lock
```

### Nodes

#### Create

If no active nodes exist, a new single node cluster will be created

```
infra node create --provider [provider] --region [region] --sku [sku] --type [type]
```

#### Delete

```
infra node destroy --name [name]
```

### External Peers

#### Add

```
infra peer add [name] --address [public IPv4]
```

This will output this peer's specific WireGuard config that can be written to
`/etc/wireguard/wg0.conf` and started with `sudo systemctl enable --now
wg-quick@wg0`

#### Remove

```
infra peer remove [name]
```

#### Generate `CertificateKey`

```
kubeadm certs certificate-key
```
SEE: [kubeadm docs](https://kubernetes.io/docs/reference/setup-tools/kubeadm/kubeadm-certs/#cmd-certs-certificate-key)
