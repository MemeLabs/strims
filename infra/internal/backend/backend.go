package backend

import (
	"bytes"
	"database/sql"
	"errors"
	"fmt"
	"io/ioutil"
	"os"
	"reflect"
	"time"

	"github.com/MemeLabs/go-ppspp/infra/pkg/node"
	"github.com/mitchellh/mapstructure"

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
}

var driverConfigType = reflect.TypeOf((*DriverConfig)(nil)).Elem()
var timeType = reflect.TypeOf(time.Time{})

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
		}
	}

	/*
		if cfg.Providers.Dreamhost != nil {
			drivers["dreamhost"], err = node.NewDreamhostDriver(cfg.Providers.Dreamhost.IdentityEndpoint, cfg.Providers.Dreamhost.TokenID, cfg.Providers.Dreamhost.TenantID)
			if err != nil {
				return nil, err
			}
		}
	*/

	return &Backend{
		Log:             log,
		DB:              db,
		NodeDrivers:     drivers,
		flake:           flake,
		sshIdentityFile: cfg.SSH.IdentityFile,
	}, nil
}

// Backend ...
type Backend struct {
	DB              *sql.DB
	Log             *zap.Logger
	NodeDrivers     map[string]node.Driver
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
