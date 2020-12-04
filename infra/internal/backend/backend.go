package backend

import (
	"bytes"
	"context"
	"database/sql"
	"errors"
	"fmt"
	"io"
	"io/ioutil"
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
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/golang/geo/s2"
	"github.com/google/go-cmp/cmp"
	"github.com/mitchellh/mapstructure"
	"github.com/volatiletech/null/v8"
	"github.com/volatiletech/sqlboiler/v4/boil"
	"github.com/volatiletech/sqlboiler/v4/queries/qm"
	"golang.org/x/crypto/ssh"

	// db driver
	_ "github.com/mattn/go-sqlite3"
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
		Path string
	}
	FlakeStartTime time.Time
	Providers      map[string]DriverConfig
	SSH            struct {
		IdentityFile string
	}
	InterfaceConfig         wgutil.InterfaceConfig
	PublicControllerAddress string
}

// TODO(jbpratt): delete this for a flag or config
const containerName = "strims-k8s"

var (
	driverConfigType = reflect.TypeOf((*DriverConfig)(nil)).Elem()
	timeType         = reflect.TypeOf(time.Time{})

	nodeSSHRetries = 100

	controllerSyncScript string
	nodeStartScript      string
	nodeSyncScript       string
)

func init() {
	h, err := os.UserHomeDir()
	if err != nil {
		panic(err)
	}

	controllerSyncScript = path.Join(h, "controller-sync-wg.py")
	nodeStartScript = path.Join(h, "node-start.sh")
	nodeSyncScript = path.Join(h, "node-sync-wg.sh")
}

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

// New creates a Backend taking in the Config
func New(cfg Config) (*Backend, error) {
	log := zap.New(zapcore.NewCore(
		zapcore.NewConsoleEncoder(zap.NewDevelopmentEncoderConfig()),
		zapcore.Lock(os.Stderr),
		zapcore.LevelEnabler(zapcore.Level(cfg.LogLevel)),
	))

	db, err := sql.Open("sqlite3", cfg.DB.Path)
	if err != nil {
		return nil, err
	}

	boil.SetDB(db)

	flake := sonyflake.NewSonyflake(sonyflake.Settings{
		StartTime: cfg.FlakeStartTime,
	})

	drivers := map[string]node.Driver{}
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

	return &Backend{
		Log:               log,
		DB:                db,
		NodeDrivers:       drivers,
		flake:             flake,
		sshIdentityFile:   cfg.SSH.IdentityFile,
		Conf:              cfg.InterfaceConfig,
		ControllerAddress: cfg.PublicControllerAddress,
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

func (b *Backend) nextWGIPv4(ctx context.Context, activeNodes []*node.Node) (string, error) {
OUTER:
	for i := 2; i < 255; i++ {
		for _, node := range activeNodes {
			addr := strings.Split(node.WireguardIPv4, ".")
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

func (b *Backend) CreateNode(
	ctx context.Context,
	driver node.Driver,
	hostname, region, sku string,
	billingType node.BillingType,
) error {

	req := &node.CreateRequest{
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

	nodes, err := b.ActiveNodes(ctx)
	if err != nil {
		return err
	}

	// append all peers from the database to static peers
	for _, node := range nodes {
		for _, peer := range b.Conf.Peers {
			if peer.Endpoint == node.Networks.V4[0] {
				continue
			}
		}

		pub, err := wgutil.PublicFromPrivate(node.WireguardPrivKey)
		if err != nil {
			// TODO: let's handle this
			continue
		}

		b.Conf.Peers = append(b.Conf.Peers, wgutil.InterfacePeerConfig{
			PublicKey:           pub,
			AllowedIPs:          fmt.Sprintf("%s/%d", node.WireguardIPv4, 32),
			Endpoint:            fmt.Sprintf("%s:%d", node.Networks.V4[0], 51820),
			PersistentKeepalive: 25,
		})
	}

	n.WireguardIPv4, err = b.nextWGIPv4(ctx, nodes)
	if n.WireguardIPv4 == "" || err != nil {
		return fmt.Errorf("failed to get next wg ipv4: %w", err)
	}

	n.WireguardPrivKey, n.WireguardPubKey, err = wgutil.GenerateKey()
	if n.WireguardPrivKey == "" || n.WireguardPubKey == "" || err != nil {
		return fmt.Errorf("failed to create wg keys: %w", err)
	}

	// append new peer
	b.Conf.Peers = append(b.Conf.Peers, wgutil.InterfacePeerConfig{
		PublicKey:           n.WireguardPubKey,
		AllowedIPs:          fmt.Sprintf("%s/%d", n.WireguardIPv4, 32),
		Endpoint:            fmt.Sprintf("%s:%d", n.Networks.V4[0], 51820),
		PersistentKeepalive: 25,
	})

	if err := b.insertNode(ctx, n, driver.Provider()); err != nil {
		return fmt.Errorf("failed to insert node(%v): %w", n, err)
	}

	if err := b.updateController(); err != nil {
		return fmt.Errorf("failed to update controller config(%v): %w", b.Conf, err)
	}

	if err := b.initNode(ctx, n, driver.DefaultUser()); err != nil {
		return fmt.Errorf("failed to init node(%v): %w", nil, err)
	}

	/*
		if err := b.syncNodes(ctx, nil); err != nil {
			return fmt.Errorf("failed to sync nodes: %w", err)
		}
	*/

	return nil
}

func (b *Backend) ActiveNodes(ctx context.Context) ([]*node.Node, error) {
	var nodes []*node.Node
	slice, err := models.Nodes(qm.Where("active=?", 1)).All(ctx, b.DB)
	if err != nil {
		return nil, err
	}

	for _, n := range slice {
		nodes = append(nodes, modelToNode(n))
	}

	return nodes, nil
}

func (b *Backend) insertNode(ctx context.Context, node *node.Node, providerName string) error {

	id, err := dao.GenerateSnowflake()
	if err != nil {
		return err
	}

	nodeEntry := &models.Node{
		ID:           int64(id),
		Active:       1,
		StartedAt:    time.Now().UnixNano(),
		ProviderID:   node.ProviderID,
		ProviderName: providerName,
		Name:         strings.ToLower(node.Name), // lowering here to match k8s node naming
		Memory:       int64(node.Memory),
		CPUs:         int64(node.CPUs),
		Disk:         int64(node.Disk),
		IPV4:         node.Networks.V4[0],
		// IPV6:       node.Networks.V6[0],
		RegionName:      node.Region.Name,
		RegionLat:       float64(node.Region.LatLng.Lat),
		RegionLng:       float64(node.Region.LatLng.Lng),
		WireguardIP:     node.WireguardIPv4,
		WireguardKey:    node.WireguardPrivKey,
		SKUPriceHourly:  node.SKU.PriceHourly.Value,
		SKUPriceMonthly: node.SKU.PriceMonthly.Value,
	}

	if err := nodeEntry.Insert(ctx, b.DB, boil.Infer()); err != nil {
		return fmt.Errorf("failed to insert node: %w", err)
	}

	return nil
}

func (b *Backend) updateController() error {

	// write the wg cfg to a temp file
	tmp, err := ioutil.TempFile("", "goppspp")
	if err != nil {
		return fmt.Errorf("failed to create tmp file: %w", err)
	}
	defer os.Remove(tmp.Name())
	if _, err := tmp.Write([]byte(b.Conf.String())); err != nil {
		return fmt.Errorf("failed to write wg conf(%s): %w", b.Conf.String(), err)
	}
	if err := tmp.Close(); err != nil {
		return fmt.Errorf("failed to close temp file: %w", err)
	}

	if err := run(
		"sudo", "-S", "python3", controllerSyncScript, tmp.Name(),
	); err != nil {
		return fmt.Errorf("failed to update controller: %w", err)
	}

	return nil
}

func (b *Backend) initNode(ctx context.Context, node *node.Node, user string) error {
	// Continuously retry ssh'ing into the new node. This is to ensure that it is
	// connectable by the time the master node begins initilization.
	// TODO(jbpratt): maybe move this as a step in node creation..
	var err error
	fmt.Print("attempting to ssh to the new node")
	for i := 0; i < nodeSSHRetries; i++ {
		fmt.Print(".")
		_, err := sshToNode(user, node.Networks.V4[0], b.SSHIdentityFile())
		if err == nil {
			break
		}
		time.Sleep(time.Second)
	}
	if err != nil {
		return fmt.Errorf("failed to connect to node: %w", err)
	}
	fmt.Print("\n")

	// construct new node's WG config
	var peers []wgutil.InterfacePeerConfig
	for _, p := range b.Conf.Peers {
		if p.Endpoint == fmt.Sprintf("%s:%d", node.Networks.V4[0], 51820) {
			continue
		}
		peers = append(peers, p)
	}

	wgPub, err := wgutil.PublicFromPrivate(b.Conf.PrivateKey)
	if wgPub == "" || err != nil {
		return fmt.Errorf("failed to determine public key from private: %w", err)
	}

	// add controller as peer
	peers = append(peers, wgutil.InterfacePeerConfig{
		AllowedIPs:          "10.0.0.1/32",
		PublicKey:           wgPub,
		Endpoint:            fmt.Sprintf("%s:%d", b.ControllerAddress, 51820),
		PersistentKeepalive: 25,
	})

	// set main interface
	conf := &wgutil.InterfaceConfig{
		PrivateKey: node.WireguardPrivKey,
		Address:    fmt.Sprintf("%s/%d", node.WireguardIPv4, 24),
		ListenPort: 51820,
		Peers:      peers,
	}

	if err := run(
		nodeStartScript,
		user,
		node.Networks.V4[0],
		b.SSHIdentityFile(),
		node.WireguardIPv4,
		conf.String(),
		node.Name,
	); err != nil {
		return fmt.Errorf("failed to exec 'node-start.sh': %w", err)
	}

	return nil
}

func (b *Backend) syncNodes(ctx context.Context, nodes []*node.Node) error {
	for _, n := range nodes {
		if err := run(
			nodeSyncScript,
			node.DefaultUser[n.Driver],
			n.Networks.V4[0],
			b.SSHIdentityFile(),
			b.Conf.String(), // TODO(jbpratt): need a different conf here
		); err != nil {
			return fmt.Errorf("failed to exec 'node-sync-wg.sh': %w", err)
		}
	}
	return nil
}

func (b *Backend) DestroyNode(ctx context.Context, name string) error {
	if err := run("sudo", "kubectl", "delete", "node", name); err != nil {
		return fmt.Errorf("failed to delete node: %w", err)
	}

	n, err := models.Nodes(qm.Where("name=?", name)).One(ctx, b.DB)
	if err != nil {
		return fmt.Errorf("failed to find node(%s) in database: %w", name, err)
	}

	// set 'stopped_at' time and 'active'
	n.Active = 0
	n.StoppedAt = null.Int64From(time.Now().Unix())

	_, err = n.Update(ctx, b.DB, boil.Infer())
	if err != nil {
		return fmt.Errorf("failed to update node: %w", err)
	}

	for i, p := range b.Conf.Peers {
		if p.PublicKey == n.IPV4 {
			b.Conf.Peers = append(b.Conf.Peers[:i], b.Conf.Peers[i+1:]...)
		}
	}

	if err := b.updateController(); err != nil {
		return fmt.Errorf("failed to update controller(%v): %w", b.Conf, err)
	}

	/*
		nodes, err := b.ActiveNodes(ctx)
		if err != nil {
			return fmt.Errorf("failed to get active nodes: %w", err)
		}

		// maybe we should be syncing by peers, what about static peers
		if err := b.syncNodes(ctx, nodes); err != nil {
			return fmt.Errorf("failed to sync nodes: %w", err)
		}
	*/

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

	dbNodes, err := b.ActiveNodes(ctx)
	if err != nil {
		return "", fmt.Errorf("failed to get active nodes: %w", err)
	}

	if diff := cmp.Diff(liveNodes, dbNodes); diff != "" {
		return diff, nil
	}

	return "", errors.New("failed to compare")
}

func modelToNode(n *models.Node) *node.Node {
	var status string
	if n.Active == 1 {
		status = "active"
	} else {
		status = "inactive"
	}

	return &node.Node{
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
		Status: status,
		Region: &node.Region{
			Name:   n.RegionName,
			LatLng: s2.LatLngFromDegrees(n.RegionLat, n.RegionLng),
		},
		SKU: &node.SKU{
			Name:         n.SKUName,
			NetworkCap:   int(n.SKUNetworkCap),
			NetworkSpeed: int(n.SKUNetworkSpeed),
			PriceMonthly: &node.Price{
				Value: n.SKUPriceMonthly,
			},
			PriceHourly: &node.Price{
				Value: n.SKUPriceHourly,
			},
		},
		WireguardPrivKey: n.WireguardKey,
		WireguardIPv4:    n.WireguardIP,
	}
}

// sshToNode allows connection to an instance via SSH. A user, address and
// private key are required, the same as using `ssh -i key.txt user@address`.
// The caller is responsible for closing the client connection.
func sshToNode(user, addr, privKeyPath string) (*ssh.Client, error) {
	key, err := ioutil.ReadFile(privKeyPath)
	if err != nil {
		return nil, err
	}
	signer, err := ssh.ParsePrivateKey(key)
	if err != nil {
		return nil, fmt.Errorf("failed to parse private key: %w", err)
	}

	conf := &ssh.ClientConfig{
		User:            user,
		Auth:            []ssh.AuthMethod{ssh.PublicKeys(signer)},
		HostKeyCallback: ssh.InsecureIgnoreHostKey(),
		Timeout:         time.Duration(10 * time.Second),
	}
	conn, err := ssh.Dial("tcp", fmt.Sprintf("%s:22", addr), conf)
	if err != nil {
		return nil, fmt.Errorf("failed to dial node(%v): %w", conf, err)
	}
	return conn, nil
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
			panic(err)
		}
	}

	go copy(os.Stderr, stderr)
	go copy(os.Stdout, stdout)

	if err := cmd.Run(); err != nil {
		return fmt.Errorf("failed to exec cmd: %w", err)
	}
	return nil
}

// copyFile copies the src file to dst. Any existing file will be overwritten
// and will not copy file attributes.
func copyFile(src, dst string) error {
	in, err := os.Open(src)
	if err != nil {
		return err
	}
	defer in.Close()

	out, err := os.Create(dst)
	if err != nil {
		return err
	}
	defer out.Close()

	_, err = io.Copy(out, in)
	if err != nil {
		return err
	}
	return out.Close()
}

func isPublicIP(IP net.IP) bool {
	if IP.IsLoopback() || IP.IsLinkLocalMulticast() || IP.IsLinkLocalUnicast() {
		return false
	}
	if ip4 := IP.To4(); ip4 != nil {
		switch {
		case ip4[0] == 10:
			return false
		case ip4[0] == 172 && ip4[1] >= 16 && ip4[1] <= 31:
			return false
		case ip4[0] == 192 && ip4[1] == 168:
			return false
		default:
			return true
		}
	}
	return false
}
