// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package main

import (
	"context"
	"flag"
	"io"
	"log"
	"net/http"
	_ "net/http/pprof"
	"os"
	"path"

	"github.com/MemeLabs/strims/internal/frontend"
	"github.com/MemeLabs/strims/internal/network"
	"github.com/MemeLabs/strims/internal/session"
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"github.com/MemeLabs/strims/pkg/errutil"
	"github.com/MemeLabs/strims/pkg/httputil"
	"github.com/MemeLabs/strims/pkg/kv/bbolt"
	"github.com/MemeLabs/strims/pkg/queue/memory"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/MemeLabs/strims/pkg/vpn"
	"github.com/prometheus/client_golang/prometheus/promhttp"
	"go.uber.org/zap"
)

var (
	profileDir    string
	addr          string
	metricsAddr   string
	rtmpAddr      string
	debugAddr     string
	webRTCPortMin uint
	webRTCPortMax uint
	vnicLabel     string
)

func init() {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		log.Fatalf("failed to locate home directory: %s", err)
	}

	flag.StringVar(&profileDir, "profile-dir", homeDir, "profile db location")
	flag.StringVar(&addr, "addr", ":8083", "bootstrap server listen address")
	flag.StringVar(&metricsAddr, "metrics-addr", ":1971", "metrics server listen address")
	flag.StringVar(&rtmpAddr, "rtmp-addr", ":1935", "rtmp server listen address")
	flag.StringVar(&debugAddr, "debug-addr", ":6060", "debug server listen address")
	flag.UintVar(&webRTCPortMin, "webrtc-port-min", 0, "webrtc ephemeral port range min")
	flag.UintVar(&webRTCPortMax, "webrtc-port-max", 0, "webrtc ephemeral port range max")
	flag.StringVar(&vnicLabel, "vnic-label", "", "vnic label")
}

func main() {
	flag.Parse()
	log.SetFlags(log.LstdFlags | log.Lshortfile | log.Lmicroseconds)

	if debugAddr != "" {
		go func() {
			log.Println(http.ListenAndServe(debugAddr, nil))
		}()
	}

	if metricsAddr != "" {
		go func() {
			http.Handle("/metrics", promhttp.Handler())
			log.Println(http.ListenAndServe(metricsAddr, nil))
		}()
	}

	logger := errutil.Must(zap.NewDevelopment())

	store, err := bbolt.NewStore(path.Join(profileDir, ".strims"))
	if err != nil {
		logger.Fatal("failed to open db", zap.Error(err))
	}

	queue := memory.NewTransport()

	newVPN := func(key *key.Key) (*vpn.Host, error) {
		ws := vnic.NewWSInterface(logger, vnic.WSInterfaceOptions{})
		wrtc := vnic.NewWebRTCInterface(vnic.NewWebRTCDialer(
			logger,
			&vnic.WebRTCDialerOptions{
				PortMin: uint16(webRTCPortMin),
				PortMax: uint16(webRTCPortMax),
			},
		))
		vnicHost, err := vnic.New(logger, key, vnic.WithInterface(ws), vnic.WithInterface(wrtc))
		if err != nil {
			return nil, err
		}
		return vpn.New(logger, vnicHost)
	}

	httpmux := httputil.NewMapServeMux()

	sessionManager := session.NewManager(logger, store, queue, newVPN, network.NewBroker(logger), httpmux)

	srv := frontend.Server{
		Store:          store,
		Logger:         logger,
		SessionManager: sessionManager,
	}

	if err := srv.Listen(context.Background(), stdio{os.Stdin, os.Stdout}); err != nil {
		logger.Fatal("frontend server closed with error", zap.Error(err))
	}

	select {}
}

type stdio struct {
	io.Reader
	io.Writer
}
