// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

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
	"sync"
	"text/template"
	"time"

	"github.com/MemeLabs/strims/infra/internal/models"
	"github.com/MemeLabs/strims/infra/pkg/node"
	"github.com/MemeLabs/strims/infra/pkg/wgutil"
	"github.com/appleboy/easyssh-proxy"
	"github.com/gofrs/flock"
	"github.com/golang/geo/s2"
	"github.com/mitchellh/mapstructure"
	"github.com/volatiletech/null/v8"
	"github.com/volatiletech/sqlboiler/v4/boil"

	// db driver
	_ "github.com/lib/pq"

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
	Providers       map[string]DriverConfig
	SSHIdentityFile string
	ScriptDirectory string
	CertificateKey  string
	Lockfile        string
}

const (
	defaultWGPort  = 51820
	wgIPRangeStart = "10.0.0.1"
)

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

	if cfg.Lockfile == "" {
		return nil, errors.New("Lockfile is required")
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
		db:              db,
		log:             log,
		sshIdentityFile: cfg.SSHIdentityFile,
		scriptDirectory: cfg.ScriptDirectory,
		certificateKey:  cfg.CertificateKey,
		lock:            flock.New(cfg.Lockfile),
	}

	return b, nil
}

// Backend ...
type Backend struct {
	NodeDrivers     map[string]node.Driver
	db              *sql.DB
	log             *zap.Logger
	scriptDirectory string
	sshIdentityFile string
	certificateKey  string
	lock            *flock.Flock
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

	nodeCount, err := b.ActiveNodeCount(ctx)
	if err != nil {
		return fmt.Errorf("failed to get active nodes: %w", err)
	}

	newCluster := false
	if nodeCount == 0 {
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

	wgpriv, wgpub, err := wgutil.GenerateKey()
	if err != nil {
		return fmt.Errorf("failed to create wg keys: %w", err)
	}
	n.WireguardPrivKey = wgpriv
	b.log.Info("generated wireguard keys for node", zap.String("wg_pub_key", wgpub))

	nodeID, err := b.insertNode(ctx, n)
	if err != nil {
		return fmt.Errorf("failed to insert node: %w", err)
	}

	n.WireguardIPv4, err = b.leaseWGIP(ctx, nodeID, models.WireguardPeerTypeNode)
	if err != nil {
		return fmt.Errorf("failed to get next wg ipv4: %w", err)
	}

	b.syncNodes(ctx)

	if err = b.initNode(ctx, n, newCluster); err != nil {
		return fmt.Errorf("failed to init node: %w", err)
	}

	if err := b.markNodeActive(ctx, nodeID); err != nil {
		return fmt.Errorf("failed to mark node active: %w", err)
	}

	if _, err := b.lock.TryLockContext(ctx, time.Second); err != nil {
		return fmt.Errorf("failed to acquire lock for final wg sync: %w", err)
	}
	defer b.lock.Unlock()
	if err := b.syncWireguard(ctx, n); err != nil {
		return fmt.Errorf("failed to mark node active: %w", err)
	}

	return nil
}

func (b *Backend) ActiveNodes(ctx context.Context) ([]*node.Node, error) {
	return b.getNodesByState(ctx, models.NodeStateActive)
}

func (b *Backend) ActiveNodeCount(ctx context.Context) (int64, error) {
	return b.getNodeCountByState(ctx, models.NodeStateActive)
}

func (b *Backend) InactiveNodes(ctx context.Context) ([]*node.Node, error) {
	return b.getNodesByState(ctx, models.NodeStateDestroyed)
}

func (b *Backend) releaseWGIP(ctx context.Context, lesseeID int64, lesseeType string) error {
	lease, err := models.FindWireguardIPLeaseG(ctx, lesseeType, lesseeID)
	if err != nil {
		return fmt.Errorf("failed to find lease: %w", err)
	}

	if _, err = lease.DeleteG(ctx); err != nil {
		return fmt.Errorf("failed to delete lease: %w", err)
	}

	return nil
}

func (b *Backend) leaseWGIP(ctx context.Context, lesseeID int64, lesseeType string) (string, error) {
	query := `
	INSERT INTO wireguard_ip_leases (
		SELECT $1, $2, min(t.ip)
		FROM (
			SELECT $3 ip
			UNION
			SELECT ip + 1
			FROM wireguard_ip_leases
		) t
		LEFT JOIN wireguard_ip_leases
		ON (t.ip = wireguard_ip_leases.ip)
		WHERE wireguard_ip_leases.ip IS NULL
	)
	RETURNING ip;`

	tx, err := b.db.Begin()
	if err != nil {
		return "", err
	}
	var ip sql.NullString
	if err := tx.QueryRow(query, lesseeType, lesseeID, wgIPRangeStart).Scan(&ip); err != nil {
		if err := tx.Rollback(); err != nil {
			return "", err
		}
		return "", err
	}
	if err := tx.Commit(); err != nil {
		return "", err
	}

	return ip.String, nil
}

func (b *Backend) getPeers(ctx context.Context) ([]*models.ExternalPeer, error) {
	peers, err := models.ExternalPeers().All(ctx, b.db)
	if err != nil {
		return nil, fmt.Errorf("failed to get external peers: %w", err)
	}
	return peers, nil
}

func (b *Backend) AddStaticPeer(ctx context.Context, name, address string, port int) (*wgutil.InterfaceConfig, error) {
	wgpriv, _, err := wgutil.GenerateKey()
	if err != nil {
		return nil, fmt.Errorf("failed to create wg keys: %w", err)
	}

	externalPeer := &models.ExternalPeer{
		Comment:             name,
		PublicIPV4:          address,
		WireguardPort:       port,
		WireguardPrivateKey: wgpriv,
	}

	if err = externalPeer.Insert(ctx, b.db, boil.Infer()); err != nil {
		return nil, fmt.Errorf("failed to insert peer: %w", err)
	}

	_, err = b.leaseWGIP(ctx, externalPeer.ID, models.WireguardPeerTypeExternalPeer)
	if err != nil {
		return nil, fmt.Errorf("failed to get next wg ipv4: %w", err)
	}

	b.syncNodes(ctx)

	return b.GetConfigForPeer(ctx, name)
}

func (b *Backend) RemoveStaticPeer(ctx context.Context, name string) error {
	peer, err := models.ExternalPeers(models.ExternalPeerWhere.Comment.EQ(name)).One(ctx, b.db)
	if err != nil {
		return fmt.Errorf("failed to find peer: %w", err)
	}

	if _, err := peer.DeleteG(ctx); err != nil {
		return fmt.Errorf("failed to find peer: %w", err)
	}

	if err = b.releaseWGIP(ctx, peer.ID, models.WireguardPeerTypeExternalPeer); err != nil {
		return fmt.Errorf("failed to release wg ip: %w", err)
	}

	b.syncNodes(ctx)

	return nil
}

func (b *Backend) getNodesByState(ctx context.Context, state ...string) ([]*node.Node, error) {
	var nodes []*node.Node
	slice, err := models.Nodes(models.NodeWhere.State.IN(state)).All(ctx, b.db)
	if err != nil {
		return nil, err
	}

	for _, n := range slice {
		node, err := modelToNode(ctx, n)
		if err != nil {
			return nil, err
		}
		nodes = append(nodes, node)
	}

	return nodes, nil
}

func (b *Backend) getNodeCountByState(ctx context.Context, state string) (int64, error) {
	return models.Nodes(models.NodeWhere.State.EQ(state)).Count(ctx, b.db)
}

func (b *Backend) insertNode(ctx context.Context, node *node.Node) (int64, error) {
	nodeEntry := &models.Node{
		Type:         node.Type.String(),
		User:         node.User,
		State:        models.NodeStateCreated,
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
		WireguardKey:    node.WireguardPrivKey,
		SKUPriceHourly:  float32(node.SKU.PriceHourly.Value),
		SKUPriceMonthly: float32(node.SKU.PriceMonthly.Value),
	}

	if err := nodeEntry.Insert(ctx, b.db, boil.Infer()); err != nil {
		return 0, fmt.Errorf("failed to insert node: %w", err)
	}

	b.log.Info("node has been inserted into the database", zap.String("name", node.Name))
	return nodeEntry.ID, nil
}

func (b *Backend) markNodeActive(ctx context.Context, nodeID int64) error {
	tx, err := b.db.Begin()
	if err != nil {
		return err
	}
	model, err := models.FindNode(ctx, tx, nodeID)
	if err != nil {
		return err
	}
	model.State = models.NodeStateActive
	if _, err := model.Update(ctx, tx, boil.Infer()); err != nil {
		return err
	}
	return tx.Commit()
}

func (b *Backend) initNode(ctx context.Context, n *node.Node, newCluster bool) error {
	b.log.Info("waiting for node to be reachable (up to 5 minutes)")
	var err error
	for i := 0; i < 5; i++ {
		if _, err = b.run(n, "whoami", 5*time.Minute); err == nil {
			b.log.Info("connected to node")
			break
		}
		time.Sleep(10 * time.Second)
	}
	if err != nil {
		return fmt.Errorf("failed to connect to node: %w", err)
	}

	if err = b.nodeSSH(n).Scp(fmt.Sprintf("%s/setup.sh", b.scriptDirectory), "/tmp/setup.sh"); err != nil {
		return fmt.Errorf("failed to copy setup.sh script to node: %w", err)
	}

	if err = b.stream(n, fmt.Sprintf("bash /tmp/setup.sh --hostname %q | tee /tmp/setup.log", n.Name)); err != nil {
		return fmt.Errorf("failed to exec 'setup.sh': %w", err)
	}

	if err = b.injectWireguardConfig(ctx, n); err != nil {
		return fmt.Errorf("failed to write new wg config: %w", err)
	}

	if _, err = b.run(n, "sudo systemctl enable --now wg-quick@wg0"); err != nil {
		return fmt.Errorf("failed to enable wg-quick@wg0 service")
	}

	if newCluster {
		b.log.Info("Creating a new cluster")
		if err = b.stream(
			n,
			fmt.Sprintf("bash /tmp/setup.sh --new --ca-key %q --public-ip %q | tee -a /tmp/setup.log", b.certificateKey, n.Networks.V4[0]),
		); err != nil {
			return fmt.Errorf("failed to exec 'setup.sh': %w", err)
		}
	} else {
		b.log.Info("Joining an existing cluster")

		// Refresh the upload-certs, they are deleted after two hours
		if _, err := b.runOnController(
			ctx,
			fmt.Sprintf("sudo kubeadm init phase upload-certs --upload-certs --certificate-key %q", b.certificateKey),
		); err != nil {
			return fmt.Errorf("failed to upload-certs: %w", err)
		}

		stdout, err := b.runOnController(ctx, "sudo kubeadm token create --print-join-command | tr -d '\n'")
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
			Provider          string
			Region            string
			SKU               string
		}{
			ApiServerEndpoint: k8sEndpoint,
			Token:             k8sToken,
			CaCertHash:        k8sCaCertHash,
			CaKey:             b.certificateKey,
			NodeName:          n.Name,
			WGIP:              n.WireguardIPv4,
			PublicIP:          n.Networks.V4[0],
			Provider:          n.ProviderName,
			Region:            n.Region.Name,
			SKU:               n.SKU.Name,
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

		if err = b.nodeSSH(n).Scp(kubeadm.Name(), "/tmp/kubeadm.yml"); err != nil {
			return fmt.Errorf("failed to scp kubeadm.yml to host: %w", err)
		}

		if err = b.stream(n, "sudo kubeadm join --v=5 --config=/tmp/kubeadm.yml"); err != nil {
			return fmt.Errorf("error running kubeadm join: %w", err)
		}

		if n.Type == node.TypeWorker {
			if _, err = b.runOnController(
				ctx,
				fmt.Sprintf("kubectl label node %q node-role.kubernetes.io/worker=worker", n.Name),
			); err != nil {
				return fmt.Errorf("failed to get kubeadm token: %w", err)
			}
		}
	}

	b.log.Info("node has been initialized", zap.String("name", n.Name))
	return nil
}

func (b *Backend) runOnController(ctx context.Context, cmd string) (string, error) {
	controllerModel, err := models.Nodes(
		models.NodeWhere.Type.EQ(models.NodeTypeController),
		models.NodeWhere.State.EQ(models.NodeStateActive),
	).One(ctx, b.db)
	if err != nil {
		return "", fmt.Errorf("failed to find a controller node: %w", err)
	}
	controller, err := modelToNode(ctx, controllerModel)
	if err != nil {
		return "", err
	}
	return b.run(controller, cmd)
}

func (b *Backend) syncNodes(ctx context.Context) {
	if _, err := b.lock.TryLockContext(ctx, time.Second); err != nil {
		b.log.Error("failed to acquire wg flock", zap.Error(err))
		return
	}
	defer b.lock.Unlock()

	nodes, err := b.ActiveNodes(ctx)
	if err != nil {
		b.log.Error("failed to get active nodes", zap.Error(err))
		return
	}

	if len(nodes) == 0 {
		b.log.Info("no nodes to sync")
		return
	}

	var wg sync.WaitGroup
	wg.Add(len(nodes))
	for _, n := range nodes {
		n := n
		go func() {
			if err := b.syncWireguard(ctx, n); err != nil {
				b.log.Error("failed to sync wireguard config for node", zap.String("name", n.Name), zap.Error(err))
			}
			wg.Done()
		}()
	}
	wg.Wait()
	b.log.Info("nodes have been synced")
}

func (b *Backend) syncWireguard(ctx context.Context, n *node.Node) error {
	if err := b.injectWireguardConfig(ctx, n); err != nil {
		return fmt.Errorf("failed to write new config: %w", err)
	}

	// TODO(jbpratt): Once Ubuntu has a newer version of wireguard, swap this
	// for: 'sudo systemctl reload wg-quick@wg0'
	if _, err := b.run(n, "sudo bash -c \"wg syncconf wg0 <(wg-quick strip wg0)\""); err != nil {
		return fmt.Errorf("failed to run syncconf: %w", err)
	}

	b.log.Info("node wireguard has been synced", zap.String("name", n.Name))
	return nil
}

func (b *Backend) injectWireguardConfig(ctx context.Context, n *node.Node) error {
	wgConf, err := b.buildWGConfig(ctx, n.Networks.V4[0], n.WireguardIPv4, n.WireguardPrivKey, defaultWGPort)
	if err != nil {
		return fmt.Errorf("failed to build config for node: %w", err)
	}

	tmp, err := tmpfile(wgConf.String())
	if err != nil {
		return fmt.Errorf("failed to create tmp file: %w", err)
	}
	defer os.Remove(tmp.Name())

	if err = b.nodeSSH(n).Scp(tmp.Name(), "/tmp/wg0.conf"); err != nil {
		return fmt.Errorf("failed to scp wg0.conf to node: %w", err)
	}

	if _, err = b.run(n, "sudo cp /tmp/wg0.conf /etc/wireguard/wg0.conf"); err != nil {
		return fmt.Errorf("failed to run mv wg0.conf: %w", err)
	}
	return nil
}

func (b *Backend) DestroyNode(ctx context.Context, name string) error {
	n, err := models.Nodes(
		models.NodeWhere.Name.EQ(name),
		models.NodeWhere.State.IN([]string{models.NodeStateCreated, models.NodeStateActive}),
	).One(ctx, b.db)
	if err != nil {
		return fmt.Errorf("failed to find node(%s) in database: %w", name, err)
	}

	n.State = models.NodeStateDestroyed
	n.StoppedAt = null.Int64From(time.Now().UnixNano())

	count, err := n.UpdateG(ctx, boil.Infer())
	if count != 1 || err != nil {
		return fmt.Errorf("failed to update node: %w", err)
	}

	b.log.Info("node has been updated in db to inactive", zap.String("name", name))

	if err = b.releaseWGIP(ctx, n.ID, models.WireguardPeerTypeNode); err != nil {
		return fmt.Errorf("failed to release wg ip: %w", err)
	}

	nodeCount, err := b.ActiveNodeCount(ctx)
	if err != nil {
		return fmt.Errorf("failed to get active nodes: %w", err)
	}

	if nodeCount >= 1 {
		if _, err = b.runOnController(
			ctx,
			fmt.Sprintf("kubectl drain %s --ignore-daemonsets --delete-emptydir-data --force --timeout=30s && kubectl delete node %s", name, name),
		); err != nil {
			b.log.Error("failed to drain and delete node", zap.Error(err))
		}
		b.log.Info("node has been deleted from the cluster", zap.String("name", name))
	}

	b.syncNodes(ctx)

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

func (b *Backend) buildWGConfig(ctx context.Context, pubIPv4, wgIPv4, privKey string, port uint64) (*wgutil.InterfaceConfig, error) {
	var peers []wgutil.InterfacePeerConfig

	nodes, err := b.getNodesByState(ctx, models.NodeStateCreated, models.NodeStateActive)
	if err != nil {
		return nil, fmt.Errorf("failed to get active nodes: %w", err)
	}
	for _, n := range nodes {
		if n.WireguardPrivKey == privKey {
			continue
		}

		wgpub, err := wgutil.PublicFromPrivate(n.WireguardPrivKey)
		if err != nil {
			return nil, fmt.Errorf("failed to create public key from private: %w", err)
		}

		peers = append(peers, wgutil.InterfacePeerConfig{
			Comment:             n.Name,
			PublicKey:           wgpub,
			AllowedIPs:          fmt.Sprintf("%s/%d", n.WireguardIPv4, 32),
			Endpoint:            fmt.Sprintf("%s:%d", n.Networks.V4[0], defaultWGPort),
			PersistentKeepalive: 25,
		})
	}

	externalPeers, err := b.getPeers(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get external peers: %w", err)
	}
	for _, p := range externalPeers {
		if p.WireguardPrivateKey == privKey {
			continue
		}

		wgpub, err := wgutil.PublicFromPrivate(p.WireguardPrivateKey)
		if err != nil {
			return nil, fmt.Errorf("failed to create public key from private: %w", err)
		}

		lease, err := models.FindWireguardIPLeaseG(ctx, models.WireguardPeerTypeExternalPeer, p.ID)
		if err != nil {
			return nil, fmt.Errorf("failed to find lease for peer: %w", err)
		}

		peers = append(peers, wgutil.InterfacePeerConfig{
			Comment:             p.Comment,
			PublicKey:           wgpub,
			AllowedIPs:          fmt.Sprintf("%s/%d", lease.IP, 32),
			Endpoint:            fmt.Sprintf("%s:%d", p.PublicIPV4, p.WireguardPort),
			PersistentKeepalive: 25,
		})
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
	peer, err := models.ExternalPeers(models.ExternalPeerWhere.Comment.EQ(name)).One(ctx, b.db)
	if err != nil {
		return nil, fmt.Errorf("failed to get external peers: %w", err)
	}

	lease, err := models.FindWireguardIPLeaseG(ctx, models.WireguardPeerTypeExternalPeer, peer.ID)
	if err != nil {
		return nil, fmt.Errorf("failed to find lease for peer: %w", err)
	}

	return b.buildWGConfig(ctx, peer.PublicIPV4, lease.IP, peer.WireguardPrivateKey, uint64(peer.WireguardPort))
}

func modelToNode(ctx context.Context, n *models.Node) (*node.Node, error) {
	var nodeType node.NodeType
	if err := nodeType.Set(n.Type); err != nil {
		return nil, err
	}

	wgip := ""
	// find lease for node
	if n.State != models.NodeStateDestroyed {
		lease, err := models.FindWireguardIPLeaseG(ctx, models.WireguardPeerTypeNode, n.ID)
		if err != nil {
			return nil, err
		}
		wgip = lease.IP
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
		Status: n.State == models.NodeStateActive,
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
		WireguardIPv4:    wgip,
		StartedAt:        n.StartedAt,
		StoppedAt:        n.StoppedAt.Int64,
	}, nil
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

func (b *Backend) nodeSSH(n *node.Node) *easyssh.MakeConfig {
	return &easyssh.MakeConfig{
		User:    n.User,
		Server:  n.Networks.V4[0],
		Port:    "22",
		Timeout: 60 * time.Second,
		KeyPath: b.SSHIdentityFile(),
	}
}

func (b *Backend) stream(n *node.Node, cmd string) error {
	stdoutChan, stderrChan, doneChan, errChan, err := b.nodeSSH(n).Stream(cmd, 10*time.Minute)
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

func (b *Backend) run(n *node.Node, cmd string, timeout ...time.Duration) (string, error) {
	stdout, stderr, _, err := b.nodeSSH(n).Run(cmd, timeout...)
	if err != nil {
		return "", fmt.Errorf("failed to run command(%q): %q %w", cmd, stderr, err)
	}
	return stdout, nil
}

var joinKubeadmConfigTpls = map[node.NodeType]string{
	node.TypeWorker: `
apiVersion: kubeadm.k8s.io/v1beta3
kind: JoinConfiguration
discovery:
  bootstrapToken:
    apiServerEndpoint: {{ .ApiServerEndpoint }}
    token: {{ .Token }}
    caCertHashes: [{{ .CaCertHash }}]
nodeRegistration:
  name: {{ .NodeName }}
  criSocket: unix://var/run/crio/crio.sock
  kubeletExtraArgs:
    node-ip: {{ .WGIP }}
    node-labels: "strims.gg/public-ip={{ .PublicIP }},strims.gg/svc=seeder,strims.gg/provider={{ .Provider }},strims.gg/region={{ .Region }},strims.gg/sku={{ .SKU }}"`,
	node.TypeController: `
apiVersion: kubeadm.k8s.io/v1beta3
kind: JoinConfiguration
discovery:
  bootstrapToken:
    apiServerEndpoint: {{ .ApiServerEndpoint }}
    token: {{ .Token }}
    caCertHashes: [{{ .CaCertHash }}]
nodeRegistration:
  name: {{ .NodeName }}
  criSocket: unix://var/run/crio/crio.sock
  kubeletExtraArgs:
    node-ip: {{ .WGIP }}
    node-labels: "strims.gg/public-ip={{ .PublicIP }},strims.gg/svc=seeder,strims.gg/provider={{ .Provider }},strims.gg/region={{ .Region }},strims.gg/sku={{ .SKU }}"
controlPlane:
  certificateKey: {{ .CaKey }}
  localAPIEndpoint:
    advertiseAddress: {{ .WGIP }}`,
}
