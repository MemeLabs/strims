package main

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"path"

	"github.com/MemeLabs/go-ppspp/pkg/bboltkv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/service"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/gorilla/websocket"
	"go.uber.org/zap"
)

func newManager(logger *zap.Logger) (*manager, error) {
	store, err := bboltkv.NewStore(path.Join(profileDir, ".strims"))
	if err != nil {
		return nil, fmt.Errorf("failed to open db: %s", err)
	}
	srv, err := service.New(service.Options{
		Store:  store,
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
		return nil, fmt.Errorf("failed to create service: %s", err)
	}

	t := &manager{
		logger:    logger,
		RPCServer: srv,
	}

	return t, nil
}

type manager struct {
	logger    *zap.Logger
	RPCServer *service.Server
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
	t.RPCServer.Listen(context.Background(), rw)
}
