// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package driver

import (
	"context"
	"io"
	"log"
	"net/http"
	"sync"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/frontend"
	"github.com/MemeLabs/strims/internal/network"
	"github.com/MemeLabs/strims/internal/session"
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"github.com/MemeLabs/strims/pkg/httputil"
	"github.com/MemeLabs/strims/pkg/kv/kvtest"
	"github.com/MemeLabs/strims/pkg/queue/memory"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/MemeLabs/strims/pkg/vpn"
	"go.uber.org/zap"
)

// NewNative ...
func NewNative() (Driver, error) {
	logger, err := zap.NewDevelopment()
	if err != nil {
		return nil, err
	}

	return &nativeDriver{logger: logger}, nil
}

type nativeDriver struct {
	logger    *zap.Logger
	clients   []nativeDriverClient
	closeOnce sync.Once
}

type nativeDriverClient struct {
	client *rpc.Client
}

func (d *nativeDriver) Client(o *ClientOptions) *rpc.Client {
	store := kvtest.NewMemStore()
	queue := memory.NewTransport()

	mux := httputil.NewMapServeMux()
	if o.VPNServerAddr != "" {
		go func() {
			err := http.ListenAndServe(o.VPNServerAddr, mux)
			d.logger.Debug("app server exited", zap.Error(err))
		}()
	}

	newVPN := func(key *key.Key) (*vpn.Host, error) {
		ws := vnic.NewWSInterface(d.logger, vnic.WSInterfaceOptions{ServeMux: mux})
		wrtc := vnic.NewWebRTCInterface(vnic.NewWebRTCDialer(d.logger, nil))
		vnicHost, err := vnic.New(d.logger, key, vnic.WithInterface(ws), vnic.WithInterface(wrtc))
		if err != nil {
			return nil, err
		}
		return vpn.New(d.logger, vnicHost)
	}

	sessionManager := session.NewManager(d.logger, store, queue, newVPN, network.NewBroker(d.logger), mux)

	srv := &frontend.Server{
		Store:          store,
		Logger:         d.logger,
		SessionManager: sessionManager,
	}

	hr, hw := io.Pipe()
	cr, cw := io.Pipe()

	go srv.Listen(context.Background(), readWriter{hr, cw})

	client, err := rpc.NewClient(d.logger, &rpc.RWDialer{
		Logger:     d.logger,
		ReadWriter: readWriter{cr, hw},
	})
	if err != nil {
		log.Fatal(err)
	}
	d.clients = append(d.clients, nativeDriverClient{client})
	return client
}

func (d *nativeDriver) Close() {
	d.closeOnce.Do(func() {
		for _, c := range d.clients {
			c.client.Close()
		}
	})
}

type readWriter struct {
	io.Reader
	io.WriteCloser
}
