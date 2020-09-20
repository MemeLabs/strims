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
	"reflect"
	"strconv"
	"strings"
	"time"

	"github.com/MemeLabs/go-ppspp/infra/internal/models"
	"github.com/MemeLabs/go-ppspp/infra/pkg/node"
	"github.com/MemeLabs/go-ppspp/infra/pkg/wgutil"
	"github.com/mitchellh/mapstructure"
	"github.com/volatiletech/sqlboiler/v4/boil"
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
	InterfaceConfig    wgutil.InterfaceConfig
	ControllerPublicIP string
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
		Log:             log,
		DB:              db,
		NodeDrivers:     drivers,
		flake:           flake,
		sshIdentityFile: cfg.SSH.IdentityFile,
		Conf:            cfg.InterfaceConfig,
	}, nil
}

// Backend ...
type Backend struct {
	DB              *sql.DB
	Log             *zap.Logger
	NodeDrivers     map[string]node.Driver
	Conf            wgutil.InterfaceConfig
	flake           *sonyflake.Sonyflake
	sshIdentityFile string
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

func (b *Backend) NextWGIPv4(ctx context.Context, activeNodes models.NodeSlice) (string, error) {
OUTER:
	for i := 1; i < 255; i++ {
		for _, node := range activeNodes {
			addr := strings.Split(node.IPV4, ".")
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

func (b *Backend) InsertNode(ctx context.Context, node *node.Node, wgIP, wgKey string) error {

	// TODO: node pricing
	nodeEntry := &models.Node{
		Active:     1,
		StartedAt:  time.Now().UnixNano(),
		ProviderID: node.ProviderID,
		Name:       node.Name,
		Memory:     int64(node.Memory),
		CPUs:       int64(node.CPUs),
		Disk:       int64(node.Disk),
		IPV4:       node.Networks.V4[0],
		// IPV6:       node.Networks.V6[0],
		RegionName:   node.Region.Name,
		RegionLat:    float64(node.Region.LatLng.Lat),
		RegionLng:    float64(node.Region.LatLng.Lng),
		WireguardIP:  wgIP,
		WireguardKey: wgKey,
	}

	if err := nodeEntry.Insert(ctx, b.DB, boil.Infer()); err != nil {
		return err
	}
	return nil
}

func (b *Backend) UpdateController() error {
	const containerName = "strims-k8s"
	const location = "/mnt/wg0.conf"

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

	// TODO: with the understanding that the final binary will be executed on
	// the lxc master we can just do the work from the python script in Go all
	// of this can be replaced with:
	/*
		const wgConf = "/etc/wireguard/wg0.conf"
		tmp, err := ioutil.TempFile("", "goppspp")
		if err != nil {
			return fmt.Errorf("failed to create tmp file: %w", err)
		}
		defer os.Remove(tmp.Name())

		// back up wireguard conf
		if err := copyFile(tmp.Name(), wgConf); err != nil {
			return fmt.Errorf("failed to copy file: (%s to %s) %v", tmp.Name(), wgConf, err)
		}

		// replace wg conf with new conf
		if err := copyFile(wgConf, newConfLocation); err != nil {
			return fmt.Errorf("failed to copy file: (%s to %s) %v", wgConf, newConfLocation, err)
		}

		if err := run("wg", "setconf", "wg0", "<(", "wg-quick", "strip", "wg0", ")"); err != nil {
			return fmt.Errorf("failed to run 'wg setconf': %v", err)
		}
	*/
	// TODO: this assumes running from lxc host which is not accurate long term.
	// This can be removed as the temp file will be written directly on the
	// container.
	if err := run(
		"lxc", "file", "push", tmp.Name(), fmt.Sprintf("%s%s", containerName, location),
	); err != nil {
		return fmt.Errorf("failed to push file to container: %w", err)
	}

	// TODO: remove `lxc exec`
	if err := run(
		"lxc", "exec", "-T", containerName, "--",
		"python3", "/mnt/controller-sync-wg.py", location,
	); err != nil {
		return fmt.Errorf("failed to update controller: %w", err)
	}
	return nil
}

func (b *Backend) controllerSync(newConfLocation string) error {

	return nil
}

func (b *Backend) InitNode(ctx context.Context, node *node.Node, user, wgIP string) error {

	// Continuously retry ssh'ing into the new node. We only do this to ensure
	// that it is connectable by the time the master node begins initilization.
	// TODO(jbpratt): maybe move this as a step in node creation..
	var err error
	for i := 0; i < nodeSSHRetries; i++ {
		fmt.Printf("trying %d\n", i)
		_, err = sshToNode(user, node.Networks.V4[0], b.SSHIdentityFile())
		if err == nil {
			break
		}
		time.Sleep(500 * time.Millisecond)
	}
	if err != nil {
		return fmt.Errorf("failed to connect to node: %w", err)
	}

	if err := run(
		"scripts/node-start.sh",
		user,
		node.Networks.V4[0],
		b.SSHIdentityFile(),
		wgIP,
		b.Conf.String(),
		node.Name,
	); err != nil {
		return fmt.Errorf("failed to exec 'node-start.sh': %w", err)
	}

	return nil
}

func (b *Backend) SyncNodes(ctx context.Context, nodes []*node.Node) error {
	for _, node := range nodes {
		if err := run(
			"scripts/node-sync-wg.sh",
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
	if output, err := exec.Command(args[0], args[1:]...).Output(); err != nil {
		return fmt.Errorf("failed to exec cmd: %w with %q", err, output)
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
