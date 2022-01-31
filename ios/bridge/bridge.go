package bridge

import (
	"context"
	"fmt"
	"io"
	"log"
	"os"
	"path"

	"github.com/MemeLabs/go-ppspp/internal/frontend"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/session"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/kv/bbolt"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
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

	newVPN := func(key *key.Key) (*vpn.Host, error) {
		ws := vnic.NewWSInterface(logger, vnic.WSInterfaceOptions{})
		wrtc := vnic.NewWebRTCInterface(vnic.NewWebRTCDialer(logger, nil))
		vnicHost, err := vnic.New(logger, key, vnic.WithInterface(ws), vnic.WithInterface(wrtc))
		if err != nil {
			return nil, err
		}
		return vpn.New(logger, vnicHost)
	}

	sessionManager := session.NewManager(logger, store, newVPN, network.NewBroker(logger))

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
