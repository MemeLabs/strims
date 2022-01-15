package main

import (
	"fmt"
	"path"

	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/kv/bbolt"
	"github.com/MemeLabs/go-ppspp/pkg/pathutil"
)

func openDB(cfg *Config) (kv.BlobStore, error) {
	switch cfg.Storage.Adapter.Get("bbolt") {
	case "bbolt":
		return bboltStorageAdapter(cfg)
	// TODO: postgres/mysql
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
