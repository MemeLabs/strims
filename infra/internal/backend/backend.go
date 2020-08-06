package backend

import (
	"database/sql"
	"errors"
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
			var driverConfig DriverConfig
			switch val.(map[string]interface{})["driver"] {
			case "DigitalOcean":
				driverConfig = &DigitalOceanConfig{}
			case "Scaleway":
				driverConfig = &ScalewayConfig{}
			case "Hetzner":
				driverConfig = &HetznerConfig{}
			default:
				return nil, errors.New("unsupported driver")
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
		}
	}

	if cfg.Providers.OVH != nil {
		driver, err := node.NewOVHDriver(node.OVHDefaultSubsidiary, cfg.Providers.OVH.AppKey, cfg.Providers.OVH.AppSecret, cfg.Providers.OVH.ConsumerSecret, cfg.Providers.OVH.ProjectID)
		if err != nil {
			return nil, err
		}

		drivers["ovh"] = driver
	}

	if cfg.Providers.Hetzner != nil {
		drivers["hetzner"] = node.NewHetznerDriver(cfg.Providers.Hetzner.Token)
	}

	if cfg.Providers.OVH != nil {
		driver, err := node.NewOVHDriver(node.OVHDefaultSubsidiary, cfg.Providers.OVH.AppKey, cfg.Providers.OVH.AppSecret, cfg.Providers.OVH.ConsumerSecret, cfg.Providers.OVH.ProjectID)
		if err != nil {
			return nil, err
		}

		drivers["ovh"] = driver
	}

	return &Backend{
		Log:         log,
		DB:          db,
		NodeDrivers: drivers,
		flake:       flake,
	}, nil
}

// Backend ...
type Backend struct {
	DB          *sql.DB
	Log         *zap.Logger
	NodeDrivers map[string]node.Driver
	flake       *sonyflake.Sonyflake
}

// NextID ...
func (b *Backend) NextID() (uint64, error) {
	return b.flake.NextID()
}
