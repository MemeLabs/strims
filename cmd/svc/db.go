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

func openDB(logger *zap.Logger, cfg StorageConfig) (kv.BlobStore, error) {
	switch cfg.Adapter.Get("bbolt") {
	case "bbolt":
		return bboltStorageAdapter(cfg)
	case "postgres":
		return postgresStorageAdapter(logger, cfg)
	default:
		return nil, fmt.Errorf("unsupported storage adapter: %s", cfg.Adapter)
	}
}

func bboltStorageAdapter(cfg StorageConfig) (kv.BlobStore, error) {
	dbPath, err := pathutil.Resolve(cfg.BBolt.Path.Get(path.Join("~", ".strims")))
	if err != nil {
		return nil, err
	}
	return bbolt.NewStore(dbPath)
}

func postgresStorageAdapter(logger *zap.Logger, cfg StorageConfig) (kv.BlobStore, error) {
	connStr := cfg.Postgres.ConnStr.Get("")
	if connStr == "" {
		return nil, errors.New("postgres conn string empty")
	}
	return postgres.NewStoreConfig(postgres.Config{
		ConnStr:       connStr,
		Logger:        logger,
		EnableLogging: cfg.Postgres.EnableLogging,
	})
}
