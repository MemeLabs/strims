// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package main

import (
	"errors"
	"fmt"

	"github.com/MemeLabs/strims/pkg/queue"
	"github.com/MemeLabs/strims/pkg/queue/memory"
	"github.com/MemeLabs/strims/pkg/queue/postgres"
	"go.uber.org/zap"
)

func openQueue(logger *zap.Logger, cfg *PeerConfig) (queue.Transport, error) {
	switch cfg.Queue.Adapter.Get("memory") {
	case "memory":
		return memoryQueueAdapter(cfg)
	case "postgres":
		return postgresQueueAdapter(logger, cfg)
	default:
		return nil, fmt.Errorf("unsupported queue adapter: %s", cfg.Queue.Adapter)
	}
}

func memoryQueueAdapter(cfg *PeerConfig) (queue.Transport, error) {
	return memory.NewTransport(), nil
}

func postgresQueueAdapter(logger *zap.Logger, cfg *PeerConfig) (queue.Transport, error) {
	connStr := cfg.Queue.Postgres.ConnStr.Get("")
	if connStr == "" {
		return nil, errors.New("postgres conn string empty")
	}
	return postgres.NewTransport(postgres.Config{
		ConnStr:       connStr,
		Logger:        logger,
		EnableLogging: cfg.Storage.Postgres.EnableLogging,
	})
}
