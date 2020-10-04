package bridge

import (
	"context"
	"fmt"
	"io"
	"log"
	"path"
	"runtime"

	"github.com/MemeLabs/go-ppspp/pkg/bboltkv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/service"
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

	kv, err := bboltkv.NewStore(path.Join(appFileLocation, ".strims"))
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
			return vpn.New(logger, vnicHost, vpn.NewBrokerFactory(logger))
		},
	})
	if err != nil {
		return nil, fmt.Errorf("error creating service: %s", err)
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
