// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package main

import (
	"errors"
	"fmt"
	"path"

	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/MemeLabs/strims/pkg/kv/bbolt"
	"github.com/MemeLabs/strims/pkg/kv/postgres"
	"github.com/MemeLabs/strims/pkg/pathutil"
	"go.uber.org/zap"
)

func openDB(logger *zap.Logger, cfg *Config) (kv.BlobStore, error) {
	switch cfg.Storage.Adapter.Get("bbolt") {
	case "bbolt":
		return bboltStorageAdapter(cfg)
	case "postgres":
		return postgresStorageAdapter(logger, cfg)
	default:
		return nil, fmt.Errorf("unsupported storage adapter: %s", cfg.Storage.Adapter)
	}
}

func bboltStorageAdapter(cfg *Config) (kv.BlobStore, error) {
	dbPath, err := pathutil.Resolve(cfg.Storage.BBolt.Path.Get(path.Join("~", ".strims")))
	if err != nil {
		return nil, err
	}
	return bbolt.NewStore(dbPath)
}

func postgresStorageAdapter(logger *zap.Logger, cfg *Config) (kv.BlobStore, error) {
	connStr := cfg.Storage.Postgres.ConnStr.Get("")
	if connStr == "" {
		return nil, errors.New("postgres conn string empty")
	}
	return postgres.NewStoreConfig(postgres.Config{
		ConnStr:       connStr,
		Logger:        logger,
		EnableLogging: cfg.Storage.Postgres.EnableLogging,
	})
}
