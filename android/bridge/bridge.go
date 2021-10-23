package bridge

import (
	"context"
	"fmt"
	"io"
	"log"
	"path"
	"runtime"

	"github.com/MemeLabs/go-ppspp/internal/frontend"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/kv/bbolt"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

type AndroidSide interface {
	EmitError(msg string)
	EmitData(b []byte)
}

func GetOs() string {
	return runtime.GOOS
}

type androidSideWriter struct {
	AndroidSide
	io.Reader
}

func (a *androidSideWriter) Write(p []byte) (int, error) {
	a.EmitData(p)
	return len(p), nil
}

type GoSide struct {
	w io.Writer
}

// Write ...
func (g *GoSide) Write(b []byte) error {
	fmt.Printf("got value %x\n", b)
	_, err := g.w.Write(b)
	return err
}

// NewGoSide ...
func NewGoSide(s AndroidSide, appFileLocation string) (*GoSide, error) {
	logger, err := zap.NewDevelopment()
	if err != nil {
		return nil, err
	}

	kv, err := bbolt.NewStore(path.Join(appFileLocation, ".strims"))
	if err != nil {
		return nil, fmt.Errorf("failed to open db: %w", err)
	}

	srv := frontend.Server{
		Store:  kv,
		Logger: logger,
		NewVPNHost: func(key *key.Key) (*vpn.Host, error) {
			vnicHost, err := vnic.New(
				logger,
				key,
				vnic.WithInterface(vnic.NewWSInterface(logger, vnic.WSInterfaceOptions{})),
				vnic.WithInterface(vnic.NewWebRTCInterface(vnic.NewWebRTCDialer(logger, nil))),
			)
			if err != nil {
				return nil, err
			}
			return vpn.New(logger, vnicHost)
		},
		Broker: network.NewBroker(logger),
	}

	inReader, inWriter := io.Pipe()

	go srv.Listen(context.Background(), &androidSideWriter{s, inReader})

	return &GoSide{
		w: inWriter,
	}, nil
}

func init() {
	log.SetFlags(log.LstdFlags | log.Lshortfile | log.Lmicroseconds)
}
