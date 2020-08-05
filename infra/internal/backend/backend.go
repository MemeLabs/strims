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
	Flake struct {
		StartTime time.Time
	}
	Providers struct {
		DigitalOcean struct {
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

	flake := sonyflake.NewSonyflake(sonyflake.Settings{
		StartTime: cfg.Flake.StartTime,
	})

	drivers := map[string]node.Driver{}

	if cfg.Providers.DigitalOcean.Token != "" {
		drivers["digitalocean"] = node.NewDigitalOceanDriver(cfg.Providers.DigitalOcean.Token)
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
