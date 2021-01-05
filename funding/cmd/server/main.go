package main

import (
	"context"
	"flag"
	"fmt"
	"io/ioutil"
	"log"
	"net/http"

	"github.com/MemeLabs/go-ppspp/funding/internal/backend"
	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/gorilla/websocket"
	_ "github.com/mattn/go-sqlite3"
	"go.uber.org/zap"
)

func main() {
	cfgPath := flag.String("path", "", "path to funding config file")
	flag.Parse()

	logger, err := zap.NewDevelopment()
	if err != nil {
		log.Fatalln("logger failed:", err)
	}

	b, err := backend.New(*cfgPath, logger)
	if err != nil {
		log.Fatalln("backend setup failed:", err)
	}

	srv := &fundingServer{
		logger: logger,
		upgrader: websocket.Upgrader{
			CheckOrigin: func(r *http.Request) bool {
				return true
			},
		},
		fundingService: &fundingService{
			backend: b,
		},
	}

	log.Println(srv.Start())
}

type fundingServer struct {
	logger         *zap.Logger
	upgrader       websocket.Upgrader
	fundingService *fundingService
}

func (s *fundingServer) Start() error {
	mux := http.NewServeMux()
	mux.HandleFunc("/api", s.handleAPI)

	s.fundingService.setupWebhooks(mux)

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
		ReadWriter: vnic.NewWSReadWriter(c),
	})

	api.RegisterFundingService(server, s.fundingService)

	server.Listen(r.Context())
}

type fundingService struct {
	backend *backend.Funding
}

func (s *fundingService) Test(ctx context.Context, req *pb.FundingTestRequest) (*pb.FundingTestResponse, error) {
	return &pb.FundingTestResponse{
		Message: fmt.Sprintf("hello, %s!", req.Name),
	}, nil
}

func (s *fundingService) GetSummary(ctx context.Context, req *pb.FundingGetSummaryRequest) (*pb.FundingGetSummaryResponse, error) {
	return &pb.FundingGetSummaryResponse{
		Summary: s.backend.Summary,
	}, nil
}

func (s *fundingService) CreateSubPlan(ctx context.Context, req *pb.FundingCreateSubPlanRequest) (*pb.FundingCreateSubPlanResponse, error) {
	id, err := s.backend.CreateSubPlan(ctx, req.Price)
	if err != nil {
		return nil, fmt.Errorf("failed to create subplan: %w", err)
	}

	return &pb.FundingCreateSubPlanResponse{SubPlanId: id}, nil
}

func (s *fundingService) setupWebhooks(m *http.ServeMux) error {
	m.HandleFunc("/hooks/transaction", s.newTransaction)
	/*
		webhooks, err := s.paypal.Client.ListWebhooks("")
		if err != nil {
			return err
		}

		if len(webhooks.Webhooks) > 0 {
			return nil
		}
	*/

	return nil
}

func (s *fundingService) newTransaction(w http.ResponseWriter, r *http.Request) {
	var webhookid string

	valid, err := s.backend.ValidWebhook(r, webhookid)
	if err != nil {
		w.WriteHeader(http.StatusUnauthorized)
		return
	}

	if !valid {
		w.WriteHeader(http.StatusUnauthorized)
		return
	}

	body, err := ioutil.ReadAll(r.Body)
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		return
	}

	// insert transaction
	if err := s.backend.InsertTransaction(body); err != nil {
		// TODO: err handle
		return
	}

	w.WriteHeader(http.StatusOK)
}
