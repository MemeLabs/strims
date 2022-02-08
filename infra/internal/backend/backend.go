// Package backend ...
package backend

import (
	"bytes"
	"context"
	"database/sql"
	_ "embed"
	"errors"
	"fmt"
	"io/ioutil"
	"math/rand"
	"os"
	"reflect"
	"strings"
	"text/template"
	"time"

	"github.com/MemeLabs/go-ppspp/infra/internal/models"
	"github.com/MemeLabs/go-ppspp/infra/pkg/node"
	"github.com/MemeLabs/go-ppspp/infra/pkg/wgutil"
	"github.com/appleboy/easyssh-proxy"
	"github.com/golang/geo/s2"
	"github.com/mitchellh/mapstructure"
	"github.com/volatiletech/null/v8"
	"github.com/volatiletech/sqlboiler/v4/boil"

	// db driver
	_ "github.com/lib/pq"
	"github.com/sony/sonyflake"

	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

// DriverConfig ...
type DriverConfig interface {
	isDriverConfig()
}

// DigitalOceanConfig ...
type DigitalOceanConfig struct {
	Token string
}

func (c *DigitalOceanConfig) isDriverConfig() {}

// ScalewayConfig ...
type ScalewayConfig struct {
	OrganizationID string
	AccessKey      string
	SecretKey      string
}

func (c *ScalewayConfig) isDriverConfig() {}

// HetznerConfig ...
type HetznerConfig struct {
	Token string
}

func (c *HetznerConfig) isDriverConfig() {}

// OVHConfig ...
type OVHConfig struct {
	AppKey      string
	AppSecret   string
	ConsumerKey string
	ProjectID   string
	Subsidiary  string
}

func (c *OVHConfig) isDriverConfig() {}

// DreamHostConfig ...
type DreamHostConfig struct {
	TenantID   string
	TenantName string
	Username   string
	Password   string
}

func (c *DreamHostConfig) isDriverConfig() {}

// HeficedConfig ...
type HeficedConfig struct {
	ClientID     string
	ClientSecret string
	TenantID     string
}

func (c *HeficedConfig) isDriverConfig() {}

// Config ...
type Config struct {
	LogLevel int
	DB       struct {
		Name string
		User string
		Pass string
		Host string
		Port int
	}
	FlakeStartTime  time.Time
	Providers       map[string]DriverConfig
	SSHIdentityFile string
	ScriptDirectory string
	CertificateKey  string
}

const defaultWGPort = 51820

var (
	driverConfigType = reflect.TypeOf((*DriverConfig)(nil)).Elem()
	timeType         = reflect.TypeOf(time.Time{})
)

// DecoderConfigOptions ...
func (c *Config) DecoderConfigOptions(config *mapstructure.DecoderConfig) {
	config.DecodeHook = mapstructure.ComposeDecodeHookFunc(
		config.DecodeHook,
		func(_, dst reflect.Type, val interface{}) (interface{}, error) {
			switch dst {
			case driverConfigType:
				valMap, ok := val.(map[string]interface{})
				if !ok {
					return nil, errors.New("invalid provider definition")
				}
				driverName, ok := valMap["driver"]
				if !ok {
					return nil, errors.New("provider definition missing driver")
				}

				var driverConfig DriverConfig
				switch driverName {
				case "DigitalOcean":
					driverConfig = &DigitalOceanConfig{}
				case "Scaleway":
					driverConfig = &ScalewayConfig{}
				case "Hetzner":
					driverConfig = &HetznerConfig{}
				case "OVH":
					driverConfig = &OVHConfig{}
				case "DreamHost":
					driverConfig = &DreamHostConfig{}
				case "Heficed":
					driverConfig = &HeficedConfig{}
				default:
					return nil, fmt.Errorf("unsupported driver: %s", driverName)
				}
				return driverConfig, mapstructure.Decode(val, driverConfig)
			case timeType:
				return time.Parse(time.RFC3339, val.(string))
			}
			return val, nil
		},
	)
}

// New ...
func New(cfg Config) (*Backend, error) {
	zapEncoderCfg := zap.NewDevelopmentEncoderConfig()
	zapEncoderCfg.EncodeTime = zapcore.TimeEncoderOfLayout("15:04:05")
	log := zap.New(zapcore.NewCore(
		zapcore.NewConsoleEncoder(zapEncoderCfg),
		zapcore.Lock(os.Stderr),
		zapcore.LevelEnabler(zapcore.Level(cfg.LogLevel)),
	))

	rand.Seed(time.Now().UnixNano())

	if len(cfg.CertificateKey) == 0 {
		return nil, errors.New("CertificateKey is required")
	}

	ctx, cancel := context.WithTimeout(context.Background(), 1*time.Second)
	defer cancel()

	connStr := fmt.Sprintf(
		"host=%s port=%d user=%s password=%s dbname=%s sslmode=disable",
		cfg.DB.Host,
		cfg.DB.Port,
		cfg.DB.User,
		cfg.DB.Pass,
		cfg.DB.Name,
	)
	db, err := sql.Open("postgres", connStr)
	if err != nil {
		return nil, fmt.Errorf("failed to open db conn: %w", err)
	}

	if err := db.PingContext(ctx); err != nil {
		return nil, fmt.Errorf("failed to ping database: %w", err)
	}

	boil.SetDB(db)

	flake := sonyflake.NewSonyflake(sonyflake.Settings{
		StartTime: cfg.FlakeStartTime,
	})

	drivers := map[string]node.Driver{"custom": node.NewCustomDriver()}
	for name, dci := range cfg.Providers {
		switch dc := dci.(type) {
		case *DigitalOceanConfig:
			drivers[name] = node.NewDigitalOceanDriver(dc.Token)
		case *ScalewayConfig:
			driver, err := node.NewScalewayDriver(dc.OrganizationID, dc.AccessKey, dc.SecretKey)
			if err != nil {
				return nil, err
			}
			drivers[name] = driver
		case *HetznerConfig:
			drivers[name] = node.NewHetznerDriver(dc.Token)
		case *OVHConfig:
			driver, err := node.NewOVHDriver(dc.Subsidiary, dc.AppKey, dc.AppSecret, dc.ConsumerKey, dc.ProjectID)
			if err != nil {
				return nil, err
			}
			drivers[name] = driver
		case *DreamHostConfig:
			driver, err := node.NewDreamHostDriver(dc.TenantID, dc.TenantName, dc.Username, dc.Password)
			if err != nil {
				return nil, err
			}
			drivers[name] = driver
		case *HeficedConfig:
			driver, err := node.NewHeficedDriver(dc.ClientID, dc.ClientSecret, dc.TenantID)
			if err != nil {
				return nil, err
			}
			drivers[name] = driver
		}
	}

	b := &Backend{
		NodeDrivers:     drivers,
		DB:              db,
		log:             log,
		flake:           flake,
		sshIdentityFile: cfg.SSHIdentityFile,
		scriptDirectory: cfg.ScriptDirectory,
		certificateKey:  cfg.CertificateKey,
		peers:           []*wgutil.InterfacePeerConfig{},
	}

	nodes, err := b.ActiveNodes(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get active nodes: %w", err)
	}

	for _, n := range nodes {
		wgpub, err := wgutil.PublicFromPrivate(n.WireguardPrivKey)
		if err != nil {
			return nil, fmt.Errorf("failed to create public key from private: %w", err)
		}
		b.peers = append(b.peers, &wgutil.InterfacePeerConfig{
			Comment:             n.Name,
			PublicKey:           wgpub,
			AllowedIPs:          fmt.Sprintf("%s/%d", n.WireguardIPv4, 32),
			Endpoint:            fmt.Sprintf("%s:%d", n.Networks.V4[0], defaultWGPort),
			PersistentKeepalive: 25,
		})
	}

	peers, err := models.ExternalPeers().AllG(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get external peers: %w", err)
	}

	for _, p := range peers {
		wgpub, err := wgutil.PublicFromPrivate(p.WireguardPrivateKey)
		if err != nil {
			return nil, fmt.Errorf("failed to create public key from private: %w", err)
		}
		b.peers = append(b.peers, &wgutil.InterfacePeerConfig{
			Comment:             p.Comment,
			PublicKey:           wgpub,
			AllowedIPs:          fmt.Sprintf("%s/%d", p.WireguardIP, 32),
			Endpoint:            fmt.Sprintf("%s:%d", p.PublicIPV4, p.WireguardPort),
			PersistentKeepalive: 25,
		})
	}

	return b, nil
}

// Backend ...
type Backend struct {
	NodeDrivers     map[string]node.Driver
	DB              *sql.DB
	log             *zap.Logger
	flake           *sonyflake.Sonyflake
	scriptDirectory string
	sshIdentityFile string
	certificateKey  string
	peers           []*wgutil.InterfacePeerConfig
}

// NextID ...
func (b *Backend) NextID() (uint64, error) {
	return b.flake.NextID()
}

// SSHIdentityFile ...
func (b *Backend) SSHIdentityFile() string {
	return b.sshIdentityFile
}

// SSHPublicKey ...
func (b *Backend) SSHPublicKey() string {
	d, err := ioutil.ReadFile(b.SSHIdentityFile() + ".pub")
	if err != nil {
		b.log.Fatal("error reading ssh public key", zap.Error(err))
	}
	return string(bytes.Trim(d, "\r\n\t "))
}

func (b *Backend) CreateNode(
	ctx context.Context,
	driver node.Driver,
	name, region, sku, user, ipv4 string,
	billingType node.BillingType,
	nodeType node.NodeType,
) error {
	b.log.Info("creating node",
		zap.String("provider", driver.Provider()),
		zap.String("name", name),
		zap.String("region", region))

	nodes, err := b.ActiveNodes(ctx)
	if err != nil {
		return fmt.Errorf("failed to get active nodes: %w", err)
	}

	newCluster := false
	if len(nodes) == 0 {
		if nodeType == node.TypeWorker {
			return errors.New("unable to provision a worker node without an existing cluster")
		}
		newCluster = true
		b.log.Info("creating a new cluster, no nodes were detected")
	}

	req := &node.CreateRequest{
		User:        user,
		IPV4:        ipv4,
		Name:        name,
		Region:      region,
		SKU:         sku,
		SSHKey:      b.SSHPublicKey(),
		BillingType: billingType,
	}

	n, err := driver.Create(ctx, req)
	if err != nil {
		return fmt.Errorf("failed to create node(%v): %w", req, err)
	}

	n.StartedAt = time.Now().UnixNano()
	n.ProviderName = driver.Provider()
	n.User = user
	n.Type = nodeType

	b.log.Info("node has been created",
		zap.String("ipv4", n.Networks.V4[0]),
		zap.String("name", n.Name))

	n.WireguardIPv4, err = b.nextWGIPv4(ctx, nodes)
	if err != nil {
		return fmt.Errorf("failed to get next wg ipv4: %w", err)
	}

	wgpriv, wgpub, err := wgutil.GenerateKey()
	if err != nil {
		return fmt.Errorf("failed to create wg keys: %w", err)
	}
	n.WireguardPrivKey = wgpriv
	b.log.Info("generated wireguard keys for node", zap.String("wg_pub_key", wgpub))

	b.peers = append(b.peers, &wgutil.InterfacePeerConfig{
		Comment:             n.Name,
		PublicKey:           wgpub,
		AllowedIPs:          fmt.Sprintf("%s/%d", n.WireguardIPv4, 32),
		Endpoint:            fmt.Sprintf("%s:%d", n.Networks.V4[0], defaultWGPort),
		PersistentKeepalive: 25,
	})

	b.syncNodes(nodes)

	if err = b.initNode(ctx, n, newCluster); err != nil {
		return fmt.Errorf("failed to init node: %w", err)
	}

	if err = b.insertNode(ctx, n); err != nil {
		return fmt.Errorf("failed to insert node: %w", err)
	}

	return nil
}

func (b *Backend) ActiveNodes(ctx context.Context) ([]*node.Node, error) {
	return b.getNodesByActivity(ctx, true)
}

func (b *Backend) InactiveNodes(ctx context.Context) ([]*node.Node, error) {
	return b.getNodesByActivity(ctx, false)
}

func (b *Backend) AddStaticPeer(ctx context.Context, name, address string, port int) (*wgutil.InterfaceConfig, error) {
	wgpriv, wgpub, err := wgutil.GenerateKey()
	if err != nil {
		return nil, fmt.Errorf("failed to create wg keys: %w", err)
	}

	nodes, err := b.ActiveNodes(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get active nodes: %w", err)
	}

	nextWGIPv4, err := b.nextWGIPv4(ctx, nodes)
	if err != nil {
		return nil, fmt.Errorf("failed to get next wg ipv4: %w", err)
	}

	if err = (&models.ExternalPeer{
		Comment:             name,
		PublicIPV4:          address,
		WireguardPort:       port,
		WireguardPrivateKey: wgpriv,
		WireguardIP:         nextWGIPv4,
	}).InsertG(ctx, boil.Infer()); err != nil {
		return nil, fmt.Errorf("failed to insert peer: %w", err)
	}

	b.peers = append(b.peers, &wgutil.InterfacePeerConfig{
		Comment:             name,
		PublicKey:           wgpub,
		AllowedIPs:          fmt.Sprintf("%s/%d", nextWGIPv4, 32),
		Endpoint:            fmt.Sprintf("%s:%d", address, port),
		PersistentKeepalive: 25,
	})

	b.syncNodes(nodes)

	return b.GetConfigForPeer(ctx, name)
}

func (b *Backend) RemoveStaticPeer(ctx context.Context, name string) error {
	if _, err := models.ExternalPeers(models.ExternalPeerWhere.Comment.EQ(name)).DeleteAllG(ctx); err != nil {
		return fmt.Errorf("failed to find peer: %w", err)
	}

	for i, p := range b.peers {
		if p.Comment == name {
			b.peers = append(b.peers[:i], b.peers[i+1:]...)
			b.log.Info("static peer has been removed from wg peers", zap.String("peer_name", name))
			break
		}
	}

	nodes, err := b.ActiveNodes(ctx)
	if err != nil {
		return fmt.Errorf("failed to get active nodes: %w", err)
	}

	b.syncNodes(nodes)

	return nil
}

func (b *Backend) getNodesByActivity(ctx context.Context, active bool) ([]*node.Node, error) {
	var nodes []*node.Node
	slice, err := models.Nodes(models.NodeWhere.Active.EQ(active)).AllG(ctx)
	if err != nil {
		return nil, err
	}

	for _, n := range slice {
		nodes = append(nodes, modelToNode(n))
	}

	return nodes, nil
}

func (b *Backend) insertNode(ctx context.Context, node *node.Node) error {
	nodeEntry := &models.Node{
		Type:         node.Type.String(),
		User:         node.User,
		Active:       true,
		StartedAt:    node.StartedAt,
		ProviderID:   node.ProviderID,
		ProviderName: node.ProviderName,
		Name:         node.Name,
		Memory:       node.Memory,
		CPUs:         node.CPUs,
		Disk:         node.Disk,
		IPV4:         node.Networks.V4[0],
		// IPV6:       node.Networks.V6[0],
		RegionName:      node.Region.Name,
		RegionLat:       float64(node.Region.LatLng.Lat),
		RegionLng:       float64(node.Region.LatLng.Lng),
		WireguardIP:     node.WireguardIPv4,
		WireguardKey:    node.WireguardPrivKey,
		SKUPriceHourly:  float32(node.SKU.PriceHourly.Value),
		SKUPriceMonthly: float32(node.SKU.PriceMonthly.Value),
	}

	if err := nodeEntry.InsertG(ctx, boil.Infer()); err != nil {
		return fmt.Errorf("failed to insert node: %w", err)
	}

	b.log.Info("node has been inserted into the database", zap.String("name", node.Name))
	return nil
}

func (b *Backend) initNode(ctx context.Context, n *node.Node, newCluster bool) error {
	ssh := &easyssh.MakeConfig{
		User:    n.User,
		Server:  n.Networks.V4[0],
		Port:    "22",
		Timeout: 60 * time.Second,
		KeyPath: b.SSHIdentityFile(),
	}

	b.log.Info("waiting for node to be reachable (up to 5 minutes)")
	var err error
	for i := 0; i < 5; i++ {
		if _, err = b.run(ssh, "whoami", 5*time.Minute); err == nil {
			b.log.Info("connected to node")
			break
		}
		time.Sleep(1 * time.Minute)
	}
	if err != nil {
		return fmt.Errorf("failed to connect to node: %w", err)
	}

	if err = ssh.Scp(fmt.Sprintf("%s/setup.sh", b.scriptDirectory), "/tmp/setup.sh"); err != nil {
		return fmt.Errorf("failed to copy setup.sh script to node: %w", err)
	}

	if err = b.stream(ssh, fmt.Sprintf("bash /tmp/setup.sh --hostname %q | tee /tmp/setup.log", n.Name)); err != nil {
		return fmt.Errorf("failed to exec 'setup.sh': %w", err)
	}

	if err = b.injectWireguardConfig(ssh, n); err != nil {
		return fmt.Errorf("failed to write new wg config: %w", err)
	}

	if err = b.stream(ssh, "sudo systemctl enable --now wg-quick@wg0"); err != nil {
		return fmt.Errorf("failed to enable wg-quick@wg0 service")
	}

	if newCluster {
		b.log.Info("Creating a new cluster")
		if err = b.stream(
			ssh,
			fmt.Sprintf("bash /tmp/setup.sh --new --ca-key %q --host-ip %q | tee -a /tmp/setup.log", b.certificateKey, n.Networks.V4[0]),
		); err != nil {
			return fmt.Errorf("failed to exec 'setup.sh': %w", err)
		}
	} else {
		b.log.Info("Joining an existing cluster")
		stdout, err := b.runOnController("sudo kubeadm token create --print-join-command | tr -d '\n'")
		if err != nil {
			return fmt.Errorf("failed to get kubeadm token: %w", err)
		}

		// kubeadm join api.kitkat.jbpratt.xyz:8443 --token h4b038.3muxan3qf8xq9rna --discovery-token-ca-cert-hash sha256:b...
		k8sJoinCmd := strings.Split(stdout, " ")
		k8sEndpoint := k8sJoinCmd[2]
		k8sToken := k8sJoinCmd[4]
		k8sCaCertHash := k8sJoinCmd[6]

		tpl, err := template.New("joinKubeadmConfig").Parse(joinKubeadmConfigTpls[n.Type])
		if err != nil {
			return fmt.Errorf("failed to parse template: %w", err)
		}

		kubeadmData := struct {
			ApiServerEndpoint string
			Token             string
			CaCertHash        string
			CaKey             string
			NodeName          string
			WGIP              string
			PublicIP          string
		}{
			ApiServerEndpoint: k8sEndpoint,
			Token:             k8sToken,
			CaCertHash:        k8sCaCertHash,
			CaKey:             b.certificateKey,
			NodeName:          n.Name,
			WGIP:              n.WireguardIPv4,
			PublicIP:          n.Networks.V4[0],
		}

		kubeadmRaw := &bytes.Buffer{}
		if err = tpl.Execute(kubeadmRaw, kubeadmData); err != nil {
			return fmt.Errorf("failed to template page: %w", err)
		}

		kubeadm, err := tmpfile(kubeadmRaw.String())
		if err != nil {
			return fmt.Errorf("failed to write raw kubeadm to tmp file: %w", err)
		}
		defer os.Remove(kubeadm.Name())

		if err = ssh.Scp(kubeadm.Name(), "/tmp/kubeadm.yml"); err != nil {
			return fmt.Errorf("failed to scp kubeadm.yml to host: %w", err)
		}

		if err = b.stream(ssh, "sudo kubeadm join --v=5 --config=/tmp/kubeadm.yml"); err != nil {
			return fmt.Errorf("error running kubeadm join: %w", err)
		}

		if n.Type == node.TypeWorker {
			if _, err = b.runOnController(
				fmt.Sprintf("kubectl label node %q node-role.kubernetes.io/worker=worker", n.Name),
			); err != nil {
				return fmt.Errorf("failed to get kubeadm token: %w", err)
			}
		}
	}

	b.log.Info("node has been initialized", zap.String("name", n.Name))
	return nil
}

func (b *Backend) runOnController(cmd string) (string, error) {
	controllerModel, err := models.Nodes(
		models.NodeWhere.Type.EQ(models.NodeTypeController),
		models.NodeWhere.Active.EQ(true),
	).OneG(context.TODO())
	if err != nil {
		return "", fmt.Errorf("failed to find a controller node: %w", err)
	}
	controller := modelToNode(controllerModel)
	return b.run(&easyssh.MakeConfig{
		User:    controller.User,
		Server:  controller.Networks.V4[0],
		Port:    "22",
		Timeout: 30 * time.Second,
		KeyPath: b.SSHIdentityFile(),
	}, cmd)
}

func (b *Backend) syncNodes(nodes []*node.Node) {
	if len(nodes) == 0 {
		b.log.Info("no nodes to sync")
		return
	}

	for _, n := range nodes {
		if err := b.syncWireguard(n); err != nil {
			b.log.Error("failed to sync wireguard config for node", zap.String("name", n.Name), zap.Error(err))
		}
	}
	b.log.Info("nodes have been synced")
}

func (b *Backend) syncWireguard(n *node.Node) error {
	ssh := &easyssh.MakeConfig{
		User:    n.User,
		Server:  n.Networks.V4[0],
		Port:    "22",
		Timeout: 60 * time.Second,
		KeyPath: b.SSHIdentityFile(),
	}

	if err := b.injectWireguardConfig(ssh, n); err != nil {
		return fmt.Errorf("failed to write new config: %w", err)
	}

	// TODO(jbpratt): Once Ubuntu has a newer version of wireguard, swap this
	// for: 'sudo systemctl reload wg-quick@wg0'
	if _, err := b.run(ssh, "sudo bash -c \"wg syncconf wg0 <(wg-quick strip wg0)\""); err != nil {
		return fmt.Errorf("failed to run syncconf: %w", err)
	}

	b.log.Info("node wireguard has been synced", zap.String("name", n.Name))
	return nil
}

func (b *Backend) injectWireguardConfig(ssh *easyssh.MakeConfig, n *node.Node) error {
	wgConf, err := b.buildWGConfig(n.Networks.V4[0], n.WireguardIPv4, n.WireguardPrivKey, defaultWGPort)
	if err != nil {
		return fmt.Errorf("failed to build config for node: %w", err)
	}

	tmp, err := tmpfile(wgConf.String())
	if err != nil {
		return fmt.Errorf("failed to create tmp file: %w", err)
	}
	defer os.Remove(tmp.Name())

	if err = ssh.Scp(tmp.Name(), "/tmp/wg0.conf"); err != nil {
		return fmt.Errorf("failed to scp wg0.conf to node: %w", err)
	}

	if _, err = b.run(ssh, "sudo cp /tmp/wg0.conf /etc/wireguard/wg0.conf"); err != nil {
		return fmt.Errorf("failed to run mv wg0.conf: %w", err)
	}
	return nil
}

func (b *Backend) DestroyNode(ctx context.Context, name string) error {
	n, err := models.Nodes(
		models.NodeWhere.Name.EQ(name),
		models.NodeWhere.Active.EQ(true),
	).OneG(context.TODO())
	if err != nil {
		return fmt.Errorf("failed to find node(%s) in database: %w", name, err)
	}

	n.Active = false
	n.StoppedAt = null.Int64From(time.Now().UnixNano())

	count, err := n.UpdateG(ctx, boil.Infer())
	if count != 1 || err != nil {
		return fmt.Errorf("failed to update node: %w", err)
	}

	b.log.Info("node has been updated in db to inactive", zap.String("name", name))

	for i, p := range b.peers {
		if p.Endpoint == fmt.Sprintf("%s:%d", n.IPV4, defaultWGPort) {
			b.peers = append(b.peers[:i], b.peers[i+1:]...)
			b.log.Info("node has been removed from wg peers", zap.String("name", name))
			break
		}
	}

	nodes, err := b.ActiveNodes(ctx)
	if err != nil {
		return fmt.Errorf("failed to get active nodes: %w", err)
	}

	if len(nodes) >= 1 {
		if _, err = b.runOnController(
			fmt.Sprintf("kubectl drain %s --ignore-daemonsets --delete-emptydir-data --force --timeout=30s && kubectl delete node %s", name, name),
		); err != nil {
			b.log.Error("failed to drain and delete node", zap.Error(err))
		}
		b.log.Info("node has been deleted from the cluster", zap.String("name", name))
	}

	b.syncNodes(nodes)

	d, ok := b.NodeDrivers[n.ProviderName]
	if !ok {
		return fmt.Errorf("unknown provider %s", n.ProviderName)
	}

	req := &node.DeleteRequest{
		Region:     n.RegionName,
		ProviderID: n.ProviderID,
	}

	if err = d.Delete(ctx, req); err != nil {
		return fmt.Errorf("failed to delete node(%v): %w", req, err)
	}

	b.log.Info("node has been deleted from the provider",
		zap.String("name", name),
		zap.String("provider", n.ProviderName))
	return nil
}

func ComputeCost(sku *node.SKU, timeOnline time.Duration) float64 {
	return sku.PriceHourly.Value * float64(timeOnline) / float64(time.Hour)
}

func (b *Backend) nextWGIPv4(ctx context.Context, activeNodes []*node.Node) (string, error) {
OUTER:
	for i := 1; i < 255; i++ {
		for _, node := range activeNodes {
			addr := strings.Split(node.WireguardIPv4, ".")
			if len(addr) == 4 {
				// addr already taken
				if addr[3] == fmt.Sprint(i) {
					continue OUTER
				}
			}
		}
		// no node has this addr
		nextWGIPv4 := fmt.Sprintf("10.0.0.%d", i)
		b.log.Info("next wireguard address", zap.String("wg_ipv4", nextWGIPv4))
		return nextWGIPv4, nil
	}
	return "", fmt.Errorf("failed to find next wg ipv4")
}

func (b *Backend) buildWGConfig(pubIPv4, wgIPv4, privKey string, port uint64) (*wgutil.InterfaceConfig, error) {
	var peers []wgutil.InterfacePeerConfig
	for _, p := range b.peers {
		// don't add ourself as a peer
		if p.Endpoint != fmt.Sprintf("%s:%d", pubIPv4, port) {
			peers = append(peers, *p)
		}
	}
	return &wgutil.InterfaceConfig{
		DNS:        "1.1.1.1",
		PrivateKey: privKey,
		Address:    fmt.Sprintf("%s/%d", wgIPv4, 24),
		ListenPort: port,
		Peers:      peers,
	}, nil
}

func (b *Backend) GetConfigForPeer(ctx context.Context, name string) (*wgutil.InterfaceConfig, error) {
	peer, err := models.ExternalPeers(models.ExternalPeerWhere.Comment.EQ(name)).OneG(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get external peers: %w", err)
	}
	return b.buildWGConfig(peer.PublicIPV4, peer.WireguardIP, peer.WireguardPrivateKey, uint64(peer.WireguardPort))
}

func modelToNode(n *models.Node) *node.Node {
	var nodeType node.NodeType
	if err := nodeType.Set(n.Type); err != nil {
		panic(err)
	}
	return &node.Node{
		Type:         nodeType,
		User:         n.User,
		ProviderName: n.ProviderName,
		ProviderID:   n.ProviderID,
		Name:         n.Name,
		Memory:       int(n.Memory),
		CPUs:         int(n.CPUs),
		Disk:         int(n.Disk),
		Networks: &node.Networks{
			V4: []string{n.IPV4},
			V6: []string{n.IPV6},
		},
		Status: n.Active,
		Region: &node.Region{
			Name:   n.RegionName,
			LatLng: s2.LatLngFromDegrees(n.RegionLat, n.RegionLng),
		},
		SKU: &node.SKU{
			Name:         n.SKUName,
			NetworkCap:   int(n.SKUNetworkCap),
			NetworkSpeed: int(n.SKUNetworkSpeed),
			PriceMonthly: &node.Price{
				Value: float64(n.SKUPriceMonthly),
			},
			PriceHourly: &node.Price{
				Value: float64(n.SKUPriceHourly),
			},
		},
		WireguardPrivKey: n.WireguardKey,
		WireguardIPv4:    n.WireguardIP,
		StartedAt:        n.StartedAt,
		StoppedAt:        n.StoppedAt.Int64,
	}
}

// caller must remove the tmpfile
func tmpfile(content string) (*os.File, error) {
	tmp, err := ioutil.TempFile("", "goppspp")
	if err != nil {
		return nil, fmt.Errorf("failed to create tmp file: %w", err)
	}
	if _, err := tmp.Write([]byte(content)); err != nil {
		return nil, fmt.Errorf("failed to write content to tmp file(%s): %w", content, err)
	}
	if err := tmp.Close(); err != nil {
		return nil, fmt.Errorf("failed to close temp file: %w", err)
	}
	return tmp, nil
}

func (b *Backend) stream(ssh *easyssh.MakeConfig, cmd string) error {
	stdoutChan, stderrChan, doneChan, errChan, err := ssh.Stream(cmd, 10*time.Minute)
	if err != nil {
		return fmt.Errorf("failed to run cmd(%q): %w", cmd, err)
	}

	isTimeout := true

loop:
	for {
		select {
		case isTimeout = <-doneChan:
			break loop
		case outline := <-stdoutChan:
			if outline != "" {
				b.log.Info(outline)
			}
		case errline := <-stderrChan:
			if errline != "" {
				if strings.HasPrefix(errline, "+") {
					b.log.Info(errline)
				} else {
					b.log.Error(errline)
				}
			}
		case err = <-errChan:
		}
	}
	if err != nil {
		return fmt.Errorf("failed to run command(%q): %w", cmd, err)
	}

	if !isTimeout {
		return fmt.Errorf("command(%q) timed out", cmd)
	}
	return nil
}

func (b *Backend) run(ssh *easyssh.MakeConfig, cmd string, timeout ...time.Duration) (string, error) {
	stdout, stderr, _, err := ssh.Run(cmd, timeout...)
	if err != nil {
		return "", fmt.Errorf("failed to run command(%q): %q %w", cmd, stderr, err)
	}
	return stdout, nil
}

var joinKubeadmConfigTpls = map[node.NodeType]string{
	node.TypeWorker: `
apiVersion: kubeadm.k8s.io/v1beta2
kind: JoinConfiguration
discovery:
  bootstrapToken:
    apiServerEndpoint: {{ .ApiServerEndpoint }}
    token: {{ .Token }}
    caCertHashes: [{{ .CaCertHash }}]
nodeRegistration:
  name: {{ .NodeName }}
  kubeletExtraArgs:
    node-ip: {{ .WGIP }}
		node-labels: "strims.gg/public-ip={{ .PublicIP }}"`,
	node.TypeController: `
apiVersion: kubeadm.k8s.io/v1beta2
kind: JoinConfiguration
discovery:
  bootstrapToken:
    apiServerEndpoint: {{ .ApiServerEndpoint }}
    token: {{ .Token }}
    caCertHashes: [{{ .CaCertHash }}]
nodeRegistration:
  name: {{ .NodeName }}
  kubeletExtraArgs:
    node-ip: {{ .WGIP }}
controlPlane:
  certificateKey: {{ .CaKey }}
  localAPIEndpoint:
    advertiseAddress: {{ .WGIP }}`,
}
