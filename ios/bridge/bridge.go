// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package bridge

import (
	"context"
	"fmt"
	"io"
	"log"
	"os"
	"path"

	"github.com/MemeLabs/strims/internal/frontend"
	"github.com/MemeLabs/strims/internal/network"
	"github.com/MemeLabs/strims/internal/session"
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"github.com/MemeLabs/strims/pkg/httputil"
	"github.com/MemeLabs/strims/pkg/kv/bbolt"
	"github.com/MemeLabs/strims/pkg/queue/memory"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/MemeLabs/strims/pkg/vpn"
	"go.uber.org/zap"
)

func init() {
	log.SetFlags(log.LstdFlags | log.Lshortfile | log.Lmicroseconds)
}

// SwiftSide ...
type SwiftSide interface {
	EmitError(msg string)
	EmitData(b []byte)
}

// NewGoSide ...
func NewGoSide(s SwiftSide) (*GoSide, error) {
	logger, err := zap.NewDevelopment()
	if err != nil {
		return nil, err
	}

	homeDir, err := os.UserHomeDir()
	if err != nil {
		return nil, fmt.Errorf("failed to locate home directory: %w", err)
	}

	store, err := bbolt.NewStore(path.Join(homeDir, "Documents", ".strims"))
	if err != nil {
		return nil, fmt.Errorf("failed to open db: %w", err)
	}

	queue := memory.NewTransport()

	newVPN := func(key *key.Key) (*vpn.Host, error) {
		ws := vnic.NewWSInterface(logger, vnic.WSInterfaceOptions{})
		wrtc := vnic.NewWebRTCInterface(logger, nil)
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

	inReader, inWriter := io.Pipe()

	go func() {
		if err := srv.Listen(context.Background(), &swiftSideReadWriter{s, inReader}); err != nil {
			logger.Fatal("frontend server closed with error", zap.Error(err))
		}
	}()

	return &GoSide{inWriter}, nil
}

type swiftSideReadWriter struct {
	SwiftSide
	io.Reader
}

func (s *swiftSideReadWriter) Write(p []byte) (int, error) {
	s.EmitData(p)
	return len(p), nil
}

// GoSide ...
type GoSide struct {
	w *io.PipeWriter
}

// Write ...
func (g *GoSide) Write(b []byte) error {
	_, err := g.w.Write(b)
	return err
}
