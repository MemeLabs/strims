package driver

import (
	"context"
	"io"
	"io/ioutil"
	"log"
	"os"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/bboltkv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/service"
	"github.com/MemeLabs/go-ppspp/pkg/services/network"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
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
	file   string
	client *rpc.Client
}

func (d *nativeDriver) Client(o *ClientOptions) *rpc.Client {
	file := tempFile()
	store, err := bboltkv.NewStore(file)
	if err != nil {
		log.Fatalf("failed to open db: %s", err)
	}

	srv, err := service.New(service.Options{
		Store:  store,
		Logger: d.logger,
		NewVPNHost: func(key *pb.Key) (*vpn.Host, error) {
			ws := vnic.NewWSInterface(d.logger, o.VPNServerAddr)
			wrtc := vnic.NewWebRTCInterface(vnic.NewWebRTCDialer(d.logger, nil))
			vnicHost, err := vnic.New(d.logger, key, vnic.WithInterface(ws), vnic.WithInterface(wrtc))
			if err != nil {
				return nil, err
			}
			return vpn.New(d.logger, vnicHost)
		},
		Broker: network.NewBroker(d.logger),
	})
	if err != nil {
		log.Fatal(err)
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
	d.clients = append(d.clients, nativeDriverClient{file, client})
	return client
}

func (d *nativeDriver) Close() {
	d.closeOnce.Do(func() {
		for _, c := range d.clients {
			os.RemoveAll(c.file)
			// c.client.Close()
		}
	})
}

type readWriter struct {
	io.Reader
	io.WriteCloser
}

func tempFile() string {
	f, err := ioutil.TempFile("", "strims-")
	if err != nil {
		panic(err)
	}
	if err := f.Close(); err != nil {
		panic(err)
	}
	return f.Name()
}
