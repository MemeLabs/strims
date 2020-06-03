package driver

import (
	"context"
	"fmt"
	"io"
	"io/ioutil"
	"log"
	"net/http"
	"os"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/service"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/gorilla/websocket"
	"go.uber.org/zap"
)

type TestDriver struct {
	file     string
	srvAddr  string
	ds       dao.MetadataStore
	store    dao.Store
	host     *rpc.Host
	Client   *rpc.Client
	Frontend *service.Frontend
	log      io.Writer
}

type Config struct {
	VpnAddr string
	SrvAddr string
	Log     io.Writer
}

var upgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool {
		return true
	},
}

func Setup(c Config) *TestDriver {

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
	client := rpc.NewClient(hw, cr)

	go func() {
		if err := host.Handle(context.Background(), cw, hr); err != nil {
			log.Fatalf("failed to setup host handler: %s", err)
		}
	}()

	return &TestDriver{
		ds:       *ds,
		store:    store,
		srvAddr:  c.SrvAddr,
		host:     host,
		Client:   client,
		Frontend: svc,
		file:     file,
		log:      c.Log,
	}
}

func (d *TestDriver) Teardown() error {
	return os.RemoveAll(d.file)
}

func (d *TestDriver) Logf(format string, a ...interface{}) {
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
	if err := os.Remove(f.Name()); err != nil {
		panic(err)
	}
	return f.Name()
}
