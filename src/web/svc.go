// +build js

package main

import (
	"context"
	"crypto/rand"
	"encoding/binary"
	"errors"
	"log"
	mrand "math/rand"
	"runtime"
	"syscall/js"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/gobridge"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/service"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/MemeLabs/go-ppspp/pkg/wasmio"
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

func init() {
	go runGC()
	seedMathRand()
}

func runGC() {
	for range time.NewTicker(10 * time.Second).C {
		runtime.GC()
	}
}

func seedMathRand() {
	var t [8]byte
	if _, err := rand.Read(t[:]); err != nil {
		panic(err)
	}
	mrand.Seed(int64(binary.LittleEndian.Uint64(t[:])))
}

func main() {
	log.SetFlags(log.LstdFlags | log.Lshortfile | log.Lmicroseconds)

	gobridge.RegisterCallback("init", func(this js.Value, args []js.Value) (interface{}, error) {
		service := args[0].String()
		bridge := args[1]

		init, ok := map[string]func(js.Value, *wasmio.Bus){
			"default": initDefault,
			"broker":  initBroker,
		}[service]
		if !ok {
			return nil, errors.New("unknown service")
		}

		go init(bridge, wasmio.NewBus(bridge, service))

		return nil, nil
	})

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

func initDefault(bridge js.Value, bus *wasmio.Bus) {
	logger := newLogger(bridge)

	srv, err := service.New(service.Options{
		Store:  wasmio.NewKVStore(bridge),
		Logger: logger,
		VPNOptions: []vpn.HostOption{
			vpn.WithNetworkBroker(vpn.NewBrokerClient(logger, wasmio.NewWorkerProxy(bridge, "broker"))),
			vpn.WithInterface(vpn.NewWSInterface(logger, bridge)),
			vpn.WithInterface(vpn.NewWebRTCInterface(vpn.NewWebRTCDialer(logger, bridge))),
		},
	})
	if err != nil {
		log.Fatalf("error creating service: %s", err)
	}

	srv.Listen(context.Background(), bus)
}

func initBroker(bridge js.Value, bus *wasmio.Bus) {
	logger := newLogger(bridge)
	svc := vpn.NewBrokerService(logger)

	rpc.NewHost(logger, svc).Listen(context.Background(), bus)
}
