package bridge

import (
	"context"
	"fmt"
	"io"
	"log"
	"os"
	"path"

	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/control/network"
	"github.com/MemeLabs/go-ppspp/pkg/frontend"
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

	kv, err := bbolt.NewStore(path.Join(homeDir, "Documents", ".strims"))
	if err != nil {
		return nil, fmt.Errorf("failed to open db: %w", err)
	}

	srv := &frontend.Server{
		Store:  kv,
		Logger: logger,
		NewVPNHost: func(key *key.Key) (*vpn.Host, error) {
			vnicHost, err := vnic.New(
				logger,
				key,
				vnic.WithInterface(vnic.NewWSInterface(logger, "")),
				vnic.WithInterface(vnic.NewWebRTCInterface(vnic.NewWebRTCDialer(logger, nil))),
			)
			if err != nil {
				return nil, err
			}
			return vpn.New(logger, vnicHost)
		},
		Broker: network.NewBroker(logger),
	}
	if err != nil {
		return nil, fmt.Errorf("error creating service: %w", err)
	}

	inReader, inWriter := io.Pipe()

	go srv.Listen(context.Background(), &swiftSideReadWriter{s, inReader})

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
