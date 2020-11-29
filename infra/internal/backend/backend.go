package backend

import (
	"bytes"
	"context"
	"database/sql"
	"errors"
	"fmt"
	"io"
	"io/ioutil"
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
	"github.com/mitchellh/mapstructure"
	"github.com/volatiletech/sqlboiler/v4/boil"
	"github.com/volatiletech/sqlboiler/v4/queries/qm"
	"golang.org/x/crypto/ssh"

	// db driver
	_ "github.com/mattn/go-sqlite3"
	"github.com/sony/sonyflake"

	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

var (
	controllerSyncScript string
	nodeStartScript      string
)

func init() {
	h, err := os.UserHomeDir()
	if err != nil {
		panic(err)
	}

	controllerSyncScript = path.Join(h, "controller-sync-wg.py")
	nodeStartScript = path.Join(h, "node-start.sh")
}

// TODO(jbpratt): delete this for a flag
const containerName = "strims-k8s"

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

var driverConfigType = reflect.TypeOf((*DriverConfig)(nil)).Elem()
var timeType = reflect.TypeOf(time.Time{})
var nodeSSHRetries = 100

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

func (b *Backend) nextWGIPv4(ctx context.Context, activeNodes models.NodeSlice) (string, error) {
OUTER:
	for i := 2; i < 255; i++ {
		for _, node := range activeNodes {
			addr := strings.Split(node.WireguardIP, ".")
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
) (*node.Node, error) {

	req := &node.CreateRequest{
		Name:        hostname,
		Region:      region,
		SKU:         sku,
		SSHKey:      b.SSHPublicKey(),
		BillingType: billingType,
	}
	n, err := driver.Create(ctx, req)
	if err != nil {
		return nil, fmt.Errorf("failed to create node(%v): %w", req, err)
	}

	b.Log.Info("node has been created")

	slice, err := models.Nodes(qm.Where("active=?", 1)).All(ctx, b.DB)
	if err != nil {
		return nil, err
	}

	// append all peers from the database to static peers
	for i := 0; i < len(slice); i++ {
		for _, peer := range b.Conf.Peers {
			if peer.Endpoint == slice[i].IPV4 {
				continue
			}
		}

		b.Conf.Peers = append(b.Conf.Peers, wgutil.InterfacePeerConfig{
			PublicKey:           slice[i].WireguardKey,
			AllowedIPs:          slice[i].WireguardIP,
			Endpoint:            fmt.Sprintf("%s:%d", slice[i].IPV4, 51820),
			PersistentKeepalive: 25,
		})
	}

	wgIPv4, err := b.nextWGIPv4(ctx, slice)
	if wgIPv4 == "" || err != nil {
		return nil, fmt.Errorf("failed to get next wg ipv4: %w", err)
	}

	b.Log.Info("nextWGIPv4", zap.String("ipv4", wgIPv4))

	wgPriv, wgPub, err := wgutil.GenerateKey()
	if wgPriv == "" || err != nil {
		return nil, fmt.Errorf("failed to create wg keys: %w", err)
	}

	// append new peer
	b.Conf.Peers = append(b.Conf.Peers, wgutil.InterfacePeerConfig{
		PublicKey:           wgPub,
		AllowedIPs:          fmt.Sprintf("%s/%d", wgIPv4, 32),
		Endpoint:            fmt.Sprintf("%s:%d", n.Networks.V4[0], 51820),
		PersistentKeepalive: 25,
	})

	n.WireguardIPv4 = wgIPv4
	n.WireguardPrivKey = wgPriv
	n.WireguardPubKey = wgPub

	return n, nil
}

func (b *Backend) InsertNode(ctx context.Context, node *node.Node) error {

	id, err := dao.GenerateSnowflake()
	if err != nil {
		return err
	}

	nodeEntry := &models.Node{
		ID:         int64(id),
		Active:     1,
		StartedAt:  time.Now().UnixNano(),
		ProviderID: node.ProviderID,
		Name:       node.Name,
		Memory:     int64(node.Memory),
		CPUs:       int64(node.CPUs),
		Disk:       int64(node.Disk),
		IPV4:       node.Networks.V4[0],
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

	b.Log.Info("node inserted into database")
	return nil
}

func (b *Backend) UpdateController() error {

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

	/*
		const wgConf = "/etc/wireguard/wg0.conf"

		// back up wireguard conf
		if err := copyFile(wgConf, "/tmp/wg0.conf"); err != nil {
			return fmt.Errorf("failed to copy file: (%s to %s) %v", "/tmp/wg0.conf", wgConf, err)
		}

		// replace wg conf with new conf
		if err := copyFile(tmp.Name(), wgConf); err != nil {
			return fmt.Errorf("failed to copy file: (%s to %s) %v", tmp.Name(), wgConf, err)
		}

		if err := run("sudo", "wg", "setconf", "wg0", "<(", "wg-quick", "strip", "wg0", ")"); err != nil {
			return fmt.Errorf("failed to run 'wg setconf': %v", err)
		}
	*/

	if err := run(
		"sudo", "-S", "python3", controllerSyncScript, tmp.Name(),
	); err != nil {
		return fmt.Errorf("failed to update controller: %w", err)
	}

	return nil
}

func (b *Backend) InitNode(ctx context.Context, node *node.Node, user string) error {

	// Continuously retry ssh'ing into the new node. This is to ensure that it is
	// connectable by the time the master node begins initilization.
	// TODO(jbpratt): maybe move this as a step in node creation..
	var err error
	for i := 0; i < nodeSSHRetries; i++ {
		fmt.Printf("trying %d\n", i)
		_, err = sshToNode(user, node.Networks.V4[0], b.SSHIdentityFile())
		if err == nil {
			break
		}
		time.Sleep(time.Second)
	}
	if err != nil {
		return fmt.Errorf("failed to connect to node: %w", err)
	}

	b.Log.Info("node ssh available")

	// construct new node's WG config
	var peers []wgutil.InterfacePeerConfig
	for _, p := range b.Conf.Peers {
		if p.Endpoint == fmt.Sprintf("%s:%d", node.Networks.V4[0], 51820) {
			continue
		}
		peers = append(peers, p)
	}

	wgPub, err := wgutil.PublicFromPrivate(b.Conf.PrivateKey)
	if err != nil {
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

func (b *Backend) SyncNodes(ctx context.Context, nodes []*node.Node) error {
	for _, node := range nodes {
		if err := run(
			//"lxc", "exec", "-T", containerName, "--", // TODO: remove `lxc exec`
			"~/node-sync-wg.sh",
			// TODO: can we just have a map of default user keyed on provider?
			"",
			node.Networks.V4[0],
			b.SSHIdentityFile(),
			b.Conf.String(),
		); err != nil {
			return fmt.Errorf("failed to exec 'node-sync-wg.sh': %w", err)
		}
	}
	return nil
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

// executes a script locally. Expects a shebang and the script to be executable
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
