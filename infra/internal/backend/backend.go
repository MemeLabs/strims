// Package backend ...
package backend

import (
	"bytes"
	"context"
	"database/sql"
	"errors"
	"fmt"
	"io"
	"io/ioutil"
	"log"
	"math/rand"
	"net"
	"os"
	"os/exec"
	"path"
	"reflect"
	"strconv"
	"strings"
	"time"

	"github.com/MemeLabs/go-ppspp/infra/internal/models"
	"github.com/MemeLabs/go-ppspp/infra/pkg/node"
	"github.com/MemeLabs/go-ppspp/infra/pkg/wgutil"
	"github.com/google/go-cmp/cmp"
	"github.com/mitchellh/mapstructure"
	"github.com/prometheus/client_golang/api"
	v1 "github.com/prometheus/client_golang/api/prometheus/v1"
	"github.com/prometheus/common/model"
	"github.com/volatiletech/null/v8"
	"github.com/volatiletech/sqlboiler/v4/boil"
	"github.com/volatiletech/sqlboiler/v4/queries/qm"
	"golang.org/x/crypto/ssh"

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
	FlakeStartTime time.Time
	Providers      map[string]DriverConfig
	SSH            struct {
		IdentityFile string
	}
	InterfaceConfig         wgutil.InterfaceConfig
	PublicControllerAddress string
	ScriptDirectory         string
	PrometheusEndpoint      string
}

var (
	driverConfigType = reflect.TypeOf((*DriverConfig)(nil)).Elem()
	timeType         = reflect.TypeOf(time.Time{})

	nodeSSHRetries = 100

	nodeStartScript string
	nodeSyncScript  string
)

// DecoderConfigOptions ...
func (c *Config) DecoderConfigOptions(config *mapstructure.DecoderConfig) {
	config.DecodeHook = mapstructure.ComposeDecodeHookFunc(config.DecodeHook, func(src, dst reflect.Type, val interface{}) (interface{}, error) {
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
	})
}

// New ...
func New(cfg Config) (*Backend, error) {
	log := zap.New(zapcore.NewCore(
		zapcore.NewConsoleEncoder(zap.NewDevelopmentEncoderConfig()),
		zapcore.Lock(os.Stderr),
		zapcore.LevelEnabler(zapcore.Level(cfg.LogLevel)),
	))

	rand.Seed(time.Now().UnixNano())

	ip, port, err := net.SplitHostPort(cfg.PublicControllerAddress)
	if ip == "" || port == "" || err != nil {
		return nil, errors.New("invalid ip address for PublicControllerAddress (ip and port required)")
	}

	ctx, cancel := context.WithTimeout(context.Background(), 1*time.Second)
	defer cancel()

	connStr := fmt.Sprintf("host=%s port=%d user=%s password=%s dbname=%s sslmode=disable", cfg.DB.Host, cfg.DB.Port, cfg.DB.User, cfg.DB.Pass, cfg.DB.Name)
	db, err := sql.Open("postgres", connStr)
	if err != nil {
		return nil, fmt.Errorf("failed to open db conn: %w", err)
	}

	if err = db.PingContext(ctx); err != nil {
		return nil, fmt.Errorf("failed to ping database: %w", err)
	}

	boil.SetDB(db)

	flake := sonyflake.NewSonyflake(sonyflake.Settings{
		StartTime: cfg.FlakeStartTime,
	})

	drivers := map[string]node.Driver{
		"custom": node.NewCustomDriver(),
		"noop":   node.NewNoopDriver(),
	}
	for name, dci := range cfg.Providers {
		var driver node.Driver
		switch dc := dci.(type) {
		case *DigitalOceanConfig:
			drivers[name] = node.NewDigitalOceanDriver(dc.Token)
		case *ScalewayConfig:
			driver, err = node.NewScalewayDriver(dc.OrganizationID, dc.AccessKey, dc.SecretKey)
			if err != nil {
				return nil, err
			}
			drivers[name] = driver
		case *HetznerConfig:
			drivers[name] = node.NewHetznerDriver(dc.Token)
		case *OVHConfig:
			driver, err = node.NewOVHDriver(dc.Subsidiary, dc.AppKey, dc.AppSecret, dc.ConsumerKey, dc.ProjectID)
			if err != nil {
				return nil, err
			}
			drivers[name] = driver
		case *DreamHostConfig:
			driver, err = node.NewDreamHostDriver(dc.TenantID, dc.TenantName, dc.Username, dc.Password)
			if err != nil {
				return nil, err
			}
			drivers[name] = driver
		case *HeficedConfig:
			driver, err = node.NewHeficedDriver(dc.ClientID, dc.ClientSecret, dc.TenantID)
			if err != nil {
				return nil, err
			}
			drivers[name] = driver
		}
	}

	if cfg.ScriptDirectory == "" {
		return nil, errors.New("config must contain script directory location")
	}

	nodeStartScript = path.Join(cfg.ScriptDirectory, "node-start.sh")
	nodeSyncScript = path.Join(cfg.ScriptDirectory, "node-sync-wg.sh")

	if _, err = os.Stat(nodeStartScript); os.IsNotExist(err) {
		return nil, fmt.Errorf("could not locate script: %s %w", nodeStartScript, err)
	}

	if _, err = os.Stat(nodeSyncScript); os.IsNotExist(err) {
		return nil, fmt.Errorf("could not locate script: %s %w", nodeSyncScript, err)
	}

	client, err := api.NewClient(api.Config{
		Address: cfg.PrometheusEndpoint,
	})
	if err != nil {
		return nil, fmt.Errorf("error creating client: %v", err)
	}

	v1api := v1.NewAPI(client)
	_, warnings, err := v1api.Query(ctx, "up", time.Now())
	if err != nil {
		log.Debug("error checking Prometheus", zap.Error(err))
	}

	if len(warnings) > 0 {
		log.Debug("prometheus 'up'", zap.Strings("warnings", warnings))
	}

	return &Backend{
		DB:                db,
		Log:               log,
		NodeDrivers:       drivers,
		Conf:              cfg.InterfaceConfig,
		flake:             flake,
		sshIdentityFile:   cfg.SSH.IdentityFile,
		ControllerAddress: cfg.PublicControllerAddress,
		v1api:             v1api,
	}, nil
}

// Backend ...
type Backend struct {
	DB                *sql.DB
	Log               *zap.Logger
	NodeDrivers       map[string]node.Driver
	Conf              wgutil.InterfaceConfig
	flake             *sonyflake.Sonyflake
	sshIdentityFile   string
	ControllerAddress string
	v1api             v1.API
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
	d, err := ioutil.ReadFile(b.sshIdentityFile + ".pub")
	if err != nil {
		b.Log.Fatal("error reading ssh public key", zap.Error(err))
	}
	return string(bytes.Trim(d, "\r\n\t "))
}

func (b *Backend) CreateNode(
	ctx context.Context,
	driver node.Driver,
	hostname, region, sku, user, ipv4 string,
	billingType node.BillingType,
) error {
	b.Log.Info("creating node",
		zap.String("provider", driver.Provider()),
		zap.String("hostname", hostname),
		zap.String("region", region))

	req := &node.CreateRequest{
		User:        user,
		IPV4:        ipv4,
		Name:        hostname,
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

	if driver.Provider() == "noop" {
		var id uint64
		id, err = b.flake.NextID()
		if err != nil {
			return err
		}
		n.ProviderId = fmt.Sprint(id)
	}

	b.Log.Info("node has been created",
		zap.String("ipv4", n.Networks.V4[0]),
		zap.String("name", n.Name))

	nodes, err := b.ActiveNodes(ctx, true)
	if err != nil {
		return err
	}

	b.buildControllerConf(nodes)

	n.WireguardIpv4, err = b.nextWGIPv4(ctx, nodes)
	if n.WireguardIpv4 == "" || err != nil {
		return fmt.Errorf("failed to get next wg ipv4: %w", err)
	}

	b.Log.Info("next wireguard address", zap.String("wg_ipv4", n.WireguardIpv4))

	wgpriv, wgpub, err := wgutil.GenerateKey()
	if wgpriv == "" || wgpub == "" || err != nil {
		return fmt.Errorf("failed to create wg keys: %w", err)
	}

	n.WireguardPrivKey = wgpriv
	if err := b.insertNode(ctx, n); err != nil {
		return fmt.Errorf("failed to insert node(%v): %w", n, err)
	}

	b.Log.Info("node has been inserted into the database")

	if driver.Provider() == "noop" {
		// return before we modify any peers or the controller
		return nil
	}

	// append new peer
	b.Conf.Peers = append(b.Conf.Peers, wgutil.InterfacePeerConfig{
		PublicKey:           wgpub,
		AllowedIPs:          fmt.Sprintf("%s/%d", n.WireguardIpv4, 32),
		Endpoint:            fmt.Sprintf("%s:%d", n.Networks.V4[0], 51820),
		PersistentKeepalive: 25,
	})

	if err := b.updateController(); err != nil {
		return fmt.Errorf("failed to update controller config(%v): %w", b.Conf, err)
	}

	b.Log.Info("controller has been updated")

	if err := b.initNode(ctx, n); err != nil {
		return fmt.Errorf("failed to init node(%v): %w", nil, err)
	}

	b.Log.Info("node has been initialized")

	b.syncNodes(ctx, nodes)

	b.Log.Info("nodes have been synced")

	return nil
}

func (b *Backend) ActiveNodes(ctx context.Context, active bool) ([]*node.Node, error) {
	var query qm.QueryMod
	if active {
		query = models.NodeWhere.StoppedAt.IsNull()
	} else {
		query = models.NodeWhere.StoppedAt.IsNotNull()
	}

	var nodes []*node.Node
	slice, err := models.Nodes(query).AllG(ctx)
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
		StartedAt:       node.StartedAt,
		StoppedAt:       null.Int64{},
		ProviderName:    node.ProviderName,
		ProviderID:      node.ProviderId,
		Name:            strings.ToLower(node.Name),
		IPV4:            node.Networks.V4[0],
		IPV6:            "",
		RegionName:      node.Region.Name,
		RegionLat:       node.Region.LatLng.Latitude,
		RegionLng:       node.Region.LatLng.Longitude,
		SKUName:         node.Sku.Name,
		SKUNetworkCap:   int(node.Sku.NetworkCap),
		SKUNetworkSpeed: int(node.Sku.NetworkSpeed),
		SKUPriceMonthly: float32(node.Sku.PriceMonthly.Value),
		SKUPriceHourly:  float32(node.Sku.PriceHourly.Value),
		SkuMemory:       int(node.Sku.Memory),
		SkuCpus:         int(node.Sku.Cpus),
		SkuDisk:         int(node.Sku.Disk),
		WireguardKey:    node.WireguardPrivKey,
		WireguardIP:     node.WireguardIpv4,
		User:            node.User,
	}

	if err := nodeEntry.InsertG(ctx, boil.Infer()); err != nil {
		return fmt.Errorf("failed to insert node: %w", err)
	}

	return nil
}

func (b *Backend) updateController() error {
	tmp, err := ioutil.TempFile("", "goppspp")
	if err != nil {
		return fmt.Errorf("failed to create tmp file: %w", err)
	}
	defer os.Remove(tmp.Name())
	if _, err := tmp.Write([]byte(b.Conf.Strip())); err != nil {
		return fmt.Errorf("failed to write wg conf(%s): %w", b.Conf.String(), err)
	}
	if err := tmp.Close(); err != nil {
		return fmt.Errorf("failed to close temp file: %w", err)
	}

	if err := run("wg", "syncconf", "wg0", tmp.Name()); err != nil {
		return fmt.Errorf("failed to run 'wg syncconf wg0': %w", err)
	}

	if err := run("wg-quick", "save", "wg0"); err != nil {
		return fmt.Errorf("failed to run 'wg-quick save wg0': %w", err)
	}

	return nil
}

func (b *Backend) initNode(ctx context.Context, node *node.Node) error {
	// Continuously retry ssh'ing into the new node. This is to ensure that it is
	// connectable by the time the master node begins initilization.
	privKey, err := ioutil.ReadFile(b.SSHIdentityFile())
	if err != nil {
		return err
	}
	signer, err := ssh.ParsePrivateKey(privKey)
	if err != nil {
		return fmt.Errorf("failed to parse private key: %w", err)
	}

	sshConf := &ssh.ClientConfig{
		User:            node.User,
		Auth:            []ssh.AuthMethod{ssh.PublicKeys(signer)},
		HostKeyCallback: ssh.InsecureIgnoreHostKey(),
		Timeout:         time.Duration(10 * time.Second),
	}

	ipv4 := node.Networks.V4[0]
	fmt.Printf("attempting to ssh to the new node(%s@%s)", node.User, ipv4)
	for i := 0; i < nodeSSHRetries; i++ {
		fmt.Print(".")

		_, err = ssh.Dial("tcp", fmt.Sprintf("%s:22", ipv4), sshConf)
		if err == nil {
			fmt.Println()
			break
		}
		time.Sleep(time.Second)
	}
	if err != nil {
		return fmt.Errorf("failed to connect to node: %w", err)
	}

	conf, err := b.configForNode(node)
	if err != nil {
		return err
	}

	if err := run(
		nodeStartScript,
		node.User,
		node.Networks.V4[0],
		b.SSHIdentityFile(),
		node.WireguardIpv4,
		conf.String(),
		node.Name,
	); err != nil {
		return fmt.Errorf("failed to exec 'node-start.sh': %w", err)
	}

	return nil
}

func (b *Backend) syncNodes(ctx context.Context, nodes []*node.Node) {
	for _, n := range nodes {
		conf, err := b.configForNode(n)
		if err != nil {
			b.Log.Error("failed to build config for node", zap.String("node_name", n.Name))
			continue
		}

		if err := run(
			nodeSyncScript,
			n.User,
			n.Networks.V4[0],
			b.SSHIdentityFile(),
			conf.String(),
		); err != nil {
			b.Log.Error("failed to sync node", zap.String("node_name", n.Name))
			continue
		}
	}
}

func (b *Backend) DestroyNode(ctx context.Context, name string) error {
	n, err := models.Nodes(qm.Where("name=?", name)).OneG(ctx)
	if err != nil {
		return fmt.Errorf("failed to find node(%s) in database: %w", name, err)
	}

	n.StoppedAt = null.Int64From(time.Now().UnixNano())

	count, err := n.UpdateG(ctx, boil.Infer())
	if count != 1 || err != nil {
		return fmt.Errorf("failed to update node: %w", err)
	}

	b.Log.Info("node has been updated in db to inactive")

	if n.ProviderName == "noop" {
		// simulate a random stop time a day within the future
		stoppedAt := time.Now().Add(time.Hour*time.Duration(rand.Intn(23)) +
			time.Minute*time.Duration(rand.Intn(59)) +
			time.Second*time.Duration(rand.Intn(59)))
		n.StoppedAt = null.Int64From(stoppedAt.UnixNano())
		return nil
	}

	for i, p := range b.Conf.Peers {
		if p.Endpoint == fmt.Sprintf("%s:%d", n.IPV4, 51820) {
			b.Conf.Peers = append(b.Conf.Peers[:i], b.Conf.Peers[i+1:]...)
			b.Log.Info("node has been removed from wg peers", zap.String("node_name", n.Name))
			break
		}
	}

	if err = run(
		"kubectl", "drain", name, "--ignore-daemonsets", "--delete-emptydir-data", "--force", "--timeout=30s",
	); err != nil {
		b.Log.Error("failed to drain node", zap.Error(err))
	}

	if err = run("kubectl", "delete", "node", name); err != nil {
		b.Log.Error("failed to delete node", zap.Error(err))
		return err
	}

	b.Log.Info("node has been deleted from kube")

	nodes, err := b.ActiveNodes(ctx, true)
	if err != nil {
		return fmt.Errorf("failed to get active nodes: %w", err)
	}

	b.buildControllerConf(nodes)

	if err := b.updateController(); err != nil {
		return fmt.Errorf("failed to update controller(%v): %w", b.Conf, err)
	}

	b.Log.Info("controller has been updated")

	b.syncNodes(ctx, nodes)

	b.Log.Info("nodes have been synced")

	d, ok := b.NodeDrivers[n.ProviderName]
	if !ok {
		return fmt.Errorf("unknown provider %s", n.ProviderName)
	}

	req := &node.DeleteRequest{
		Region:     n.RegionName,
		ProviderID: n.ProviderID,
	}

	if err := d.Delete(ctx, req); err != nil {
		return fmt.Errorf("failed to delete node(%v): %w", req, err)
	}

	b.Log.Info("node has been deleted at the provider", zap.String("provider", n.ProviderName))

	return nil
}

func (b *Backend) DiffNodes(ctx context.Context) (string, error) {
	var liveNodes []*node.Node
	for _, driver := range b.NodeDrivers {
		res, err := driver.List(ctx, nil)
		if err != nil {
			return "", fmt.Errorf("failed to list nodes for %q: %w", driver.Provider(), err)
		}

		liveNodes = append(liveNodes, res...)
	}

	dbNodes, err := b.ActiveNodes(ctx, true)
	if err != nil {
		return "", fmt.Errorf("failed to get active nodes: %w", err)
	}

	if diff := cmp.Diff(liveNodes, dbNodes); diff != "" {
		return diff, nil
	}

	return "", nil
}

func (b *Backend) nextWGIPv4(ctx context.Context, activeNodes []*node.Node) (string, error) {
OUTER:
	for i := 2; i < 255; i++ {
		for _, node := range activeNodes {
			addr := strings.Split(node.WireguardIpv4, ".")
			if len(addr) == 4 {
				host, err := strconv.Atoi(addr[3])
				if err != nil {
					return "", fmt.Errorf("failed to parse ipv4 addr(%q): %w", addr[3], err)
				}

				// addr already taken
				if host == i {
					continue OUTER
				}
			}
		}
		// no node has this addr
		return fmt.Sprintf("10.0.0.%d", i), nil
	}
	return "", fmt.Errorf("failed to find next wg ipv4")
}

func (b *Backend) buildControllerConf(nodes []*node.Node) {
	b.Conf.Peers = nil
	for _, node := range nodes {
		for _, peer := range b.Conf.Peers {
			if peer.Endpoint == node.Networks.V4[0] {
				continue
			}
		}

		pub, err := wgutil.PublicFromPrivate(node.WireguardPrivKey)
		if err != nil {
			b.Log.Error("failed to get public key from node's private key",
				zap.String("node_name", node.Name))
			continue
		}

		b.Conf.Peers = append(b.Conf.Peers, wgutil.InterfacePeerConfig{
			PublicKey:           pub,
			AllowedIPs:          fmt.Sprintf("%s/%d", node.WireguardIpv4, 32),
			Endpoint:            fmt.Sprintf("%s:%d", node.Networks.V4[0], 51820),
			PersistentKeepalive: 25,
		})
	}
}

func (b *Backend) configForNode(n *node.Node) (*wgutil.InterfaceConfig, error) {
	const wgport = 51820

	var peers []wgutil.InterfacePeerConfig
	for _, p := range b.Conf.Peers {
		if p.Endpoint == fmt.Sprintf("%s:%d", n.Networks.V4[0], wgport) {
			continue
		}
		peers = append(peers, p)
	}

	wgPub, err := wgutil.PublicFromPrivate(b.Conf.PrivateKey)
	if wgPub == "" || err != nil {
		return nil, fmt.Errorf("failed to determine public key from private: %w", err)
	}

	// add controller as peer
	peers = append(peers, wgutil.InterfacePeerConfig{
		AllowedIPs:          "10.0.0.1/32",
		PublicKey:           wgPub,
		Endpoint:            b.ControllerAddress,
		PersistentKeepalive: 25,
	})

	// set main interface
	return &wgutil.InterfaceConfig{
		PrivateKey: n.WireguardPrivKey,
		Address:    fmt.Sprintf("%s/%d", n.WireguardIpv4, 24),
		ListenPort: wgport,
		Peers:      peers,
	}, nil
}

func ComputeCost(sku *node.SKU, timeOnline time.Duration) float64 {
	return sku.PriceHourly.Value * float64(timeOnline) / float64(time.Hour)
}

func modelToNode(n *models.Node) *node.Node {
	var status string
	if n.StoppedAt.IsZero() {
		status = "active"
	} else {
		status = "inactive"
	}

	return &node.Node{
		User:         n.User,
		ProviderName: n.ProviderName,
		ProviderId:   n.ProviderID,
		Name:         n.Name,
		Networks: &node.Networks{
			V4: []string{n.IPV4},
			V6: []string{n.IPV6},
		},
		Status: status,
		Region: &node.Region{
			Name:   n.RegionName,
			LatLng: node.LatLngFromDegrees(n.RegionLat, n.RegionLng),
		},
		Sku: &node.SKU{
			Memory:       int32(n.SkuMemory),
			Cpus:         int32(n.SkuCpus),
			Disk:         int32(n.SkuDisk),
			Name:         n.SKUName,
			NetworkCap:   int32(n.SKUNetworkCap),
			NetworkSpeed: int32(n.SKUNetworkSpeed),
			PriceMonthly: &node.Price{
				Value:    float64(n.SKUPriceMonthly),
				Currency: "",
			},
			PriceHourly: &node.Price{
				Value:    float64(n.SKUPriceHourly),
				Currency: "",
			},
		},
		WireguardPrivKey: n.WireguardKey,
		WireguardIpv4:    n.WireguardIP,
		StartedAt:        n.StartedAt,
		StoppedAt:        n.StoppedAt.Int64, // TODO(jbpratt): this may not be safe
	}
}

// executes a shell command
func run(args ...string) error {
	fmt.Printf("+ %q\n", strings.Join(args, " "))
	cmd := exec.Command(args[0], args[1:]...)

	stderr, err := cmd.StderrPipe()
	if err != nil {
		return err
	}
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return err
	}

	copy := func(w io.Writer, r io.Reader) {
		if _, err := io.Copy(w, r); err != nil {
			log.Printf("error while copying: %s\n", err.Error())
		}
	}

	go copy(os.Stderr, stderr)
	go copy(os.Stdout, stdout)

	if err := cmd.Run(); err != nil {
		return fmt.Errorf("failed to exec cmd: %w", err)
	}

	return nil
}

func (b *Backend) SyncNodeStats(ctx context.Context) error {
	b.Log.Debug("syncing node stats")

	slice, err := models.Nodes(models.NodeWhere.StoppedAt.IsNull()).AllG(ctx)
	if err != nil {
		return fmt.Errorf("failed to query nodes: %w", err)
	}

	usages := make(map[string]*models.Usage, len(slice))
	for _, n := range slice {
		usages[n.WireguardIP] = &models.Usage{
			NodeID: n.ID,
			Time:   time.Now().UnixNano(),
		}
	}

	const memoryQuery = "100 * (1 - ((avg_over_time(node_memory_MemFree_bytes[15m]) + avg_over_time(node_memory_Cached_bytes[15m]) + avg_over_time(node_memory_Buffers_bytes[15m])) / avg_over_time(node_memory_MemTotal_bytes[15m])))"
	// query node stats, group by name
	res, warnings, err := b.v1api.Query(ctx, memoryQuery, time.Now())
	if err != nil {
		b.Log.Debug("error checking Prometheus", zap.Error(err))
		return fmt.Errorf("memory query failed: %w", err)
	}

	if len(warnings) > 0 {
		b.Log.Debug("prometheus mem query", zap.Strings("warnings", warnings))
	}

	vec, ok := res.(model.Vector)
	if !ok {
		return errors.New("wrong response type")
	}

	for _, v := range vec {
		wgIPv4 := strings.Split(string(v.Metric["instance"]), ":")[0]
		var u *models.Usage
		u, ok = usages[wgIPv4]
		if !ok {
			// TODO: handle this
			continue
		}
		u.Mem = float64(v.Value)
	}

	const cpuQuery = `100 - (avg by (instance) (rate(node_cpu_seconds_total{mode="idle"}[15m])) * 100)`
	res, warnings, err = b.v1api.Query(ctx, cpuQuery, time.Now())
	if err != nil {
		b.Log.Debug("error checking Prometheus", zap.Error(err))
		return fmt.Errorf("cpu query failed: %w", err)
	}

	if len(warnings) > 0 {
		b.Log.Debug("prometheus mem query", zap.Strings("warnings", warnings))
	}

	vec, ok = res.(model.Vector)
	if !ok {
		return errors.New("wrong response type")
	}

	for _, v := range vec {
		wgIPv4 := strings.Split(string(v.Metric["instance"]), ":")[0]
		var u *models.Usage
		u, ok = usages[wgIPv4]
		if !ok {
			// TODO: handle this
			continue
		}
		u.CPU = float64(v.Value)
	}

	const networkBytesInQuery = "sum by (instance) (node_network_receive_bytes_total)"
	const networkBytesOutQuery = "sum by (instance) (node_network_receive_bytes_total)"

	// improve this
	for _, u := range usages {
		if err = u.InsertG(ctx, boil.Infer()); err != nil {
			b.Log.Error("failed to insert usage entry", zap.Error(err))
		}
	}

	b.Log.Info("inserted latest usage stats")

	return nil
}
