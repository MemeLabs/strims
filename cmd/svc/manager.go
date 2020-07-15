package main

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"os"
	"path"

	"github.com/MemeLabs/go-ppspp/pkg/bboltkv"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/service"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/gorilla/websocket"
	"go.uber.org/zap"
)

func newManager(logger *zap.Logger) (*manager, error) {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		return nil, fmt.Errorf("failed to locate home directory: %s", err)
	}
	store, err := bboltkv.NewStore(path.Join(homeDir, ".strims"))
	if err != nil {
		return nil, fmt.Errorf("failed to open db: %s", err)
	}
	svc, err := service.New(service.Options{
		Store: store,
		VPNOptions: []vpn.HostOption{
			vpn.WithInterface(vpn.NewWSInterface(logger, "0.0.0.0:8082")),
		},
	})
	if err != nil {
		return nil, fmt.Errorf("failed to create service: %s", err)
	}

	t := &manager{
		RPCService: rpc.NewHost(svc),
	}

	return t, nil
}

type manager struct {
	logger     *zap.Logger
	RPCService *rpc.Host
}

func (t *manager) Run() {
	mux := http.NewServeMux()
	mux.HandleFunc("/manage", t.manage)

	srv := http.Server{
		Addr:    "0.0.0.0:8083",
		Handler: mux,
	}
	log.Println("starting server at", srv.Addr)
	log.Println(srv.ListenAndServe())
}

var upgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool {
		return true
	},
}

func (t *manager) manage(w http.ResponseWriter, r *http.Request) {
	log.Println("connection received")
	c, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Print("upgrade:", err)
		return
	}

	rw := vpn.NewWSReadWriter(c)
	if err := t.RPCService.Handle(context.Background(), rw, rw); err != nil {
		log.Printf("connection closed: %s", err)
	}
}
