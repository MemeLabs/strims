package driver

import (
	"context"
	"fmt"
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

type Driver struct {
	file   string
	ds     dao.MetadataStore
	store  dao.Store
	host   *rpc.Host
	Client *rpc.Client
	log    io.Writer
}

type Config struct {
	VpnAddr string
	Log     io.Writer
}

func Setup(c Config) *Driver {

	logger, err := zap.NewDevelopment()
	if err != nil {
		panic(err)
	}

	file := tempfile()

	store, err := kv.NewKVStore(file)
	if err != nil {
		log.Fatalf("failed to open db: %s", err)
	}

	ds, err := dao.NewMetadataStore(store)
	if err != nil {
		panic(err)
	}

	svc, err := service.New(service.Options{
		Store: store,
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
	go host.Handle(context.Background(), cw, hr)
	client := rpc.NewClient(hw, cr)

	return &Driver{
		ds:     *ds,
		store:  store,
		host:   host,
		Client: client,
		file:   file,
		log:    c.Log,
	}
}

func (d *Driver) Teardown() error {
	return os.RemoveAll(d.file)
}

func (d *Driver) Logf(format string, a ...interface{}) {
	fmt.Fprintf(d.log, format+"\n", a...)
}

func tempfile() string {
	f, err := ioutil.TempFile("", "strims-")
	if err != nil {
		panic(err)
	}
	if err := f.Close(); err != nil {
		panic(err)
	}
	return f.Name()
}
