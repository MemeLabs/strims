package driver

import (
	"context"
	"io"
	"io/ioutil"
	"log"
	"os"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/service"
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

func (d *nativeDriver) Client() *rpc.Client {
	file := tempFile()
	store, err := kv.NewKVStore(file)
	if err != nil {
		log.Fatalf("failed to open db: %s", err)
	}

	svc, err := service.New(service.Options{
		Store:  store,
		Logger: d.logger,
		VPNOptions: []vpn.HostOption{
			vpn.WithNetworkBroker(vpn.NewNetworkBroker()),
			vpn.WithInterface(vpn.NewWSInterface(d.logger, "0.0.0.0:8082")),
			vpn.WithInterface(vpn.NewWebRTCInterface(&vpn.WebRTCDialer{})),
		},
	})
	if err != nil {
		log.Fatal(err)
	}

	host := rpc.NewHost(svc)
	hr, hw := io.Pipe()
	cr, cw := io.Pipe()

	go host.Handle(context.Background(), cw, hr)

	client := rpc.NewClient(hw, cr)

	d.clients = append(d.clients, nativeDriverClient{file, client})

	return client
}

func (d *nativeDriver) Close() {
	d.closeOnce.Do(func() {
		for _, c := range d.clients {
			os.RemoveAll(c.file)
			c.client.Close()
		}
	})
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
