package bridge

import (
	"context"
	"fmt"
	"io"
	"log"
	"os"
	"path"

	"github.com/MemeLabs/go-ppspp/pkg/bboltkv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/service"
	"github.com/MemeLabs/go-ppspp/pkg/services/network"
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

	kv, err := bboltkv.NewStore(path.Join(homeDir, "Documents", ".strims"))
	if err != nil {
		return nil, fmt.Errorf("failed to open db: %w", err)
	}

	srv, err := service.New(service.Options{
		Store:  kv,
		Logger: logger,
		NewVPNHost: func(key *pb.Key) (*vpn.Host, error) {
			ws := vnic.NewWSInterface(logger, "")
			wrtc := vnic.NewWebRTCInterface(vnic.NewWebRTCDialer(logger, nil))
			vnicHost, err := vnic.New(logger, key, vnic.WithInterface(ws), vnic.WithInterface(wrtc))
			if err != nil {
				return nil, err
			}
			return vpn.New(logger, vnicHost, network.NewBrokerFactory(logger))
		},
	})
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
