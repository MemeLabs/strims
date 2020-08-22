package bridge

import (
	"context"
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
func NewGoSide(s SwiftSide) *GoSide {
	logger, err := zap.NewDevelopment()
	if err != nil {
		panic(err)
	}

	homeDir, err := os.UserHomeDir()
	if err != nil {
		logger.Fatal("failed to locate home directory", zap.Error(err))
	}

	kv, err := bboltkv.NewStore(path.Join(homeDir, ".strims"))
	if err != nil {
		logger.Fatal("failed to open db", zap.Error(err))
	}

	svc, err := service.New(service.Options{
		Store:  kv,
		Logger: logger,
		VPNOptions: []vpn.HostOption{
			vpn.WithNetworkBroker(vpn.NewNetworkBroker(logger)),
			vpn.WithInterface(vpn.NewWSInterface(logger, "")),
			vpn.WithInterface(vpn.NewWebRTCInterface(vpn.NewWebRTCDialer(logger))),
		},
	})
	if err != nil {
		log.Fatalf("error creating service: %s", err)
	}

	inReader, inWriter := io.Pipe()

	go rpc.NewHost(logger, svc).Handle(context.Background(), &swiftSideWriter{s}, inReader)

	return &GoSide{
		w: inWriter,
	}
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
func (g *GoSide) Write(b []byte) {
	if _, err := g.w.Write(b); err != nil {
		panic(err)
	}
}
