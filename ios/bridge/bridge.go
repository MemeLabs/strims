package bridge

import (
	"context"
	"fmt"
	"io"
	"log"
	"net/http"
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
			vpn.WithInterface(vpn.NewWebRTCInterface(vpn.NewWebRTCDialer(logger))),
		},
	})
	if err != nil {
		return nil, fmt.Errorf("error creating service: %w", err)
	}

	inReader, inWriter := io.Pipe()

	go rpc.NewHost(logger, svc).Handle(context.Background(), &swiftSideWriter{s}, inReader)

	go runHTTPProxyThing()

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

func runHTTPProxyThing() {
	s := &http.Server{
		Addr: "127.0.0.1:8003",
		Handler: http.HandlerFunc(func(res http.ResponseWriter, req *http.Request) {
			subres, err := http.Get(fmt.Sprintf("http://192.168.0.111:8000%s", req.URL.Path))
			if err != nil {
				panic(err)
			}
			res.Header().Add("content-type", subres.Header.Get("content-type"))
			res.Header().Add("content-length", subres.Header.Get("content-length"))
			io.Copy(res, subres.Body)
			subres.Body.Close()
		}),
	}
	s.ListenAndServe()
}
