package backend

import (
	"database/sql"
	"os"
	"time"

	"github.com/MemeLabs/go-ppspp/infra/pkg/node"
	// db driver
	_ "github.com/mattn/go-sqlite3"
	"github.com/sony/sonyflake"

	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

// Config ...
type Config struct {
	LogLevel int
	DB       struct {
		Path string
	}
	FlakeStartTime string
	Providers      struct {
		DigitalOcean *struct {
			Token string
		}
		Scaleway *struct {
			OrganizationID string
			AccessKey      string
			SecretKey      string
		}
		Hetzner *struct {
			Token string
		}
	}
	SSH struct {
		IdentityFile string
	}
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

	startTime, err := time.Parse(time.RFC3339, cfg.FlakeStartTime)
	if err != nil {
		return nil, err
	}
	flake := sonyflake.NewSonyflake(sonyflake.Settings{
		StartTime: startTime,
	})

	drivers := map[string]node.Driver{}

	if cfg.Providers.DigitalOcean != nil {
		drivers["digitalocean"] = node.NewDigitalOceanDriver(cfg.Providers.DigitalOcean.Token)
	}

	if cfg.Providers.Scaleway != nil {
		driver, err := node.NewScalewayDriver(cfg.Providers.Scaleway.OrganizationID, cfg.Providers.Scaleway.AccessKey, cfg.Providers.Scaleway.SecretKey)
		if err != nil {
			return nil, err
		}
		drivers["scaleway"] = driver
	}

	if cfg.Providers.Hetzner != nil {
		drivers["hetzner"] = node.NewHetznerDriver(cfg.Providers.Hetzner.Token)
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
