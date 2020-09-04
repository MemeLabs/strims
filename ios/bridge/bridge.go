package bridge

import (
	"context"
	"fmt"
	"io"
	"log"
	"os"
	"path"

	"github.com/MemeLabs/go-ppspp/pkg/bboltkv"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/service"
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

	svc, err := service.New(service.Options{
		Store:  kv,
		Logger: logger,
		VPNOptions: []vpn.HostOption{
			vpn.WithNetworkBroker(vpn.NewNetworkBroker(logger)),
			vpn.WithInterface(vpn.NewWSInterface(logger, "")),
			vpn.WithInterface(vpn.NewWebRTCInterface(vpn.NewWebRTCDialer(logger, nil))),
		},
	})
	if err != nil {
		return nil, fmt.Errorf("error creating service: %w", err)
	}

	inReader, inWriter := io.Pipe()

	go rpc.NewHost(logger, svc).Handle(context.Background(), &swiftSideWriter{s}, inReader)

	return &GoSide{inWriter}, nil
}

type swiftSideWriter struct {
	SwiftSide
}

func (s *swiftSideWriter) Write(p []byte) (int, error) {
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
