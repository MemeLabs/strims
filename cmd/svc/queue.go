// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package main

import (
	"errors"
	"fmt"

	"github.com/MemeLabs/strims/pkg/queue"
	"github.com/MemeLabs/strims/pkg/queue/memory"
	"github.com/MemeLabs/strims/pkg/queue/postgres"
)

func openQueue(cfg *Config) (queue.Transport, error) {
	switch cfg.Queue.Adapter.Get("memory") {
	case "memory":
		return memoryQueueAdapter(cfg)
	case "postgres":
		return postgresQueueAdapter(cfg)
	default:
		return nil, fmt.Errorf("unsupported storage adapter: %s", cfg.Queue.Adapter)
	}
}

func memoryQueueAdapter(cfg *Config) (queue.Transport, error) {
	return memory.NewTransport(), nil
}

func postgresQueueAdapter(cfg *Config) (queue.Transport, error) {
	connStr := cfg.Queue.Postgres.ConnStr.Get("")
	if connStr == "" {
		return nil, errors.New("postgres conn string empty")
	}
	return postgres.NewTransport(connStr)
}
