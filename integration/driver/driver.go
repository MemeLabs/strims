package driver

import (
	"context"
	"io"
	"io/ioutil"
	"log"
	"os"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/service"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

type TestDriver struct {
	File     string
	Store    dao.Store
	host     *rpc.Host
	Client   *rpc.Client
	Frontend *service.Frontend
}

type Config struct {
	VpnAddr string
	File    string
	Store   dao.Store
}

func Setup(c Config) *TestDriver {

	logger, err := zap.NewDevelopment()
	if err != nil {
		panic(err)
	}

	if c.Store == nil && c.File == "" {
		file := tempfile()
		store, err := kv.NewKVStore(file)
		if err != nil {
			log.Fatalf("failed to open db: %s", err)
		}
		c.Store = store
		c.File = file
	}

	svc, err := service.New(service.Options{
		Store: c.Store,
		VPNOptions: []vpn.HostOption{
			vpn.WithInterface(vpn.NewWSInterface(logger, c.VpnAddr)),
		},
	})
	if err != nil {
		log.Fatal(err)
	}

	host := rpc.NewHost(svc)
	hr, hw := io.Pipe()
	cr, cw := io.Pipe()
	client := rpc.NewClient(hw, cr)

	go func() {
		if err := host.Handle(context.Background(), cw, hr); err != nil {
			log.Fatalf("failed to setup host handler: %s", err)
		}
	}()

	return &TestDriver{
		Store:    c.Store,
		host:     host,
		Client:   client,
		Frontend: svc,
		File:     c.File,
	}
}

func (d *TestDriver) Teardown() error {
	return os.RemoveAll(d.File)
}

func tempfile() string {
	f, err := ioutil.TempFile("", "strims-")
	if err != nil {
		panic(err)
	}
	if err := f.Close(); err != nil {
		panic(err)
	}
	if err := os.Remove(f.Name()); err != nil {
		panic(err)
	}
	return f.Name()
}
