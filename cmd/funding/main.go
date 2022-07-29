// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package main

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"os"
	"path"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	funding "github.com/MemeLabs/strims/pkg/apis/funding/v1"
	"github.com/MemeLabs/strims/pkg/httputil"
	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/MemeLabs/strims/pkg/kv/bbolt"
	"github.com/gorilla/websocket"
	"go.uber.org/zap"
)

func main() {
	logger, err := zap.NewDevelopment()
	if err != nil {
		log.Fatalln("logger failed:", err)
	}

	homeDir, err := os.UserHomeDir()
	if err != nil {
		log.Fatalln("locaing home directory failed:", err)
	}

	store, err := bbolt.NewStore(path.Join(homeDir, ".strims"))
	if err != nil {
		log.Fatalln("opening db failed:", err)
	}

	srv := &fundingServer{
		logger: logger,
		upgrader: websocket.Upgrader{
			CheckOrigin: func(r *http.Request) bool {
				return true
			},
		},
		service: &fundingService{
			store: store,
		},
	}

	log.Println(srv.Start())
}

type fundingServer struct {
	logger   *zap.Logger
	upgrader websocket.Upgrader
	service  *fundingService
}

func (s *fundingServer) Start() error {
	mux := http.NewServeMux()
	mux.HandleFunc("/api", s.handleAPI)

	srv := http.Server{
		Addr:    "0.0.0.0:8084",
		Handler: mux,
	}
	s.logger.Debug("starting server", zap.String("addr", srv.Addr))
	return srv.ListenAndServe()
}

func (s *fundingServer) handleAPI(w http.ResponseWriter, r *http.Request) {
	c, err := s.upgrader.Upgrade(w, r, nil)
	if err != nil {
		s.logger.Debug("upgrade failed", zap.Error(err))
		return
	}

	server := rpc.NewServer(s.logger, &rpc.RWDialer{
		Logger:     s.logger,
		ReadWriter: httputil.NewDefaultWSReadWriter(c),
	})

	funding.RegisterFundingService(server, s.service)

	server.Listen(r.Context())
}

type fundingService struct {
	store kv.BlobStore
}

func (s *fundingService) Test(ctx context.Context, req *funding.FundingTestRequest) (*funding.FundingTestResponse, error) {
	return &funding.FundingTestResponse{
		Message: fmt.Sprintf("hello, %s!", req.Name),
	}, nil
}
