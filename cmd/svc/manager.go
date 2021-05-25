package main

import (
	"fmt"
	"log"
	"net/http"
	"path"

	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/control/network"
	"github.com/MemeLabs/go-ppspp/pkg/frontend"
	"github.com/MemeLabs/go-ppspp/pkg/kv/bbolt"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/gorilla/websocket"
	"go.uber.org/zap"
)

func newManager(logger *zap.Logger) (*manager, error) {
	store, err := bbolt.NewStore(path.Join(profileDir, ".strims"))
	if err != nil {
		return nil, fmt.Errorf("failed to open db: %s", err)
	}
	srv := &frontend.Server{
		Store:  store,
		Logger: logger,
		NewVPNHost: func(key *key.Key) (*vpn.Host, error) {
			ws := vnic.NewWSInterface(logger, "")
			wrtc := vnic.NewWebRTCInterface(vnic.NewWebRTCDialer(logger, nil))
			vnicHost, err := vnic.New(logger, key, vnic.WithInterface(ws), vnic.WithInterface(wrtc))
			if err != nil {
				return nil, err
			}
			return vpn.New(logger, vnicHost)
		},
		Broker: network.NewBroker(logger),
	}

	t := &manager{
		logger:    logger,
		RPCServer: srv,
	}

	return t, nil
}

type manager struct {
	logger    *zap.Logger
	RPCServer *frontend.Server
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

	rw := vnic.NewWSReadWriter(c)
	t.RPCServer.Listen(r.Context(), rw)
}
