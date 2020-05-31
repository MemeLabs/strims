package driver

import (
	"context"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"path"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/service"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

type TestDriver struct {
	dir    string
	ds     dao.MetadataStore
	store  dao.Store
	host   *rpc.Host
	client *rpc.Client
	log    io.Writer
}

type Config struct {
	SrvAddr string
	VpnAddr string
	Log     io.Writer
}

func Setup(c Config) *TestDriver {

	logger, err := zap.NewDevelopment()
	if err != nil {
		panic(err)
	}

	tempDir := os.TempDir()

	go func() {
		log.Println(http.ListenAndServe(c.SrvAddr, nil))
	}()

	store, err := kv.NewKVStore(path.Join(tempDir, ".strims"))
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

	return &TestDriver{
		ds:     *ds,
		store:  store,
		host:   host,
		client: client,
		dir:    tempDir,
		log:    c.Log,
	}
}

func (d *TestDriver) Teardown() error {

	return os.RemoveAll(d.dir)
}

func (d *TestDriver) Logf(format string, a ...interface{}) {
	fmt.Fprintf(d.log, format+"\n", a...)
}
