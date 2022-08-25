// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build js

package main

import (
	"context"
	"errors"
	"log"
	"math/rand"
	"runtime"
	"syscall/js"
	"time"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/frontend"
	"github.com/MemeLabs/strims/internal/network"
	"github.com/MemeLabs/strims/internal/session"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"github.com/MemeLabs/strims/pkg/errutil"
	"github.com/MemeLabs/strims/pkg/gobridge"
	"github.com/MemeLabs/strims/pkg/httputil"
	"github.com/MemeLabs/strims/pkg/queue/memory"
	"github.com/MemeLabs/strims/pkg/randutil"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/MemeLabs/strims/pkg/vpn"
	"github.com/MemeLabs/strims/pkg/wasmio"
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

func init() {
	go runGC()
	rand.Seed(int64(errutil.Must(randutil.Uint64())))
}

func runGC() {
	for range time.NewTicker(10 * time.Second).C {
		runtime.GC()
	}
}

func main() {
	log.SetFlags(log.LstdFlags | log.Lshortfile | log.Lmicroseconds)

	gobridge.RegisterCallback("init", func(this js.Value, args []js.Value) (any, error) {
		service := args[0].String()
		bridge := args[1]

		init, ok := map[string]func(js.Value, wasmio.Bus){
			"default": initDefault,
			"broker":  initBroker,
		}[service]
		if !ok {
			return nil, errors.New("unknown service")
		}

		go init(bridge, wasmio.NewBus(bridge, service))

		return nil, nil
	})

	gobridge.MarkLive()

	select {}
}

func newLogger(bridge js.Value) *zap.Logger {
	encoderCfg := zapcore.EncoderConfig{
		MessageKey:     "msg",
		LevelKey:       "level",
		NameKey:        "logger",
		CallerKey:      "caller",
		EncodeLevel:    zapcore.LowercaseLevelEncoder,
		EncodeTime:     zapcore.ISO8601TimeEncoder,
		EncodeDuration: zapcore.StringDurationEncoder,
		EncodeCaller:   zapcore.ShortCallerEncoder,
	}
	sink := wasmio.NewZapSink(bridge)
	core := zapcore.NewCore(zapcore.NewJSONEncoder(encoderCfg), sink, zap.DebugLevel)
	return zap.New(core, zap.WithCaller(true))
}

func initDefault(bridge js.Value, bus wasmio.Bus) {
	logger := newLogger(bridge)
	dao.Logger = logger

	store := wasmio.NewKVStore(bridge)
	queue := memory.NewTransport()

	broker, err := network.NewBrokerProxyClient(logger, wasmio.NewWorkerProxy(bridge, "broker"))
	if err != nil {
		logger.Fatal("broker proxy init failed", zap.Error(err))
	}

	newVPN := func(key *key.Key) (*vpn.Host, error) {
		ws := vnic.NewWSInterface(logger, bridge)
		wrtc := vnic.NewWebRTCInterface(logger, bridge)
		vnicHost, err := vnic.New(logger, key, vnic.WithInterface(ws), vnic.WithInterface(wrtc))
		if err != nil {
			return nil, err
		}
		return vpn.New(logger, vnicHost)
	}

	// TODO: expose via service worker
	httpmux := httputil.NewMapServeMux()

	sessionManager := session.NewManager(logger, store, queue, newVPN, broker, httpmux)

	srv := frontend.Server{
		Store:          store,
		Logger:         logger,
		SessionManager: sessionManager,
	}

	if err := srv.Listen(context.Background(), bus); err != nil {
		logger.Fatal("frontend server closed with error", zap.Error(err))
	}
}

func initBroker(bridge js.Value, bus wasmio.Bus) {
	logger := newLogger(bridge)
	dao.Logger = logger

	server := rpc.NewServer(logger, &rpc.RWDialer{
		Logger:     logger,
		ReadWriter: bus,
	})

	networkv1.RegisterBrokerProxyService(server, network.NewBrokerProxyService(logger))

	if err := server.Listen(context.Background()); err != nil {
		logger.Fatal("network broker proxy server closed with error", zap.Error(err))
	}
}
