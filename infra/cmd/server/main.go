package main

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"log"
	"net/http"
	"time"

	be "github.com/MemeLabs/go-ppspp/infra/internal/backend"
	infrav1 "github.com/MemeLabs/go-ppspp/pkg/apis/infra/v1"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/gorilla/websocket"
	"github.com/spf13/viper"
	"github.com/volatiletech/sqlboiler/boil"
	"go.uber.org/zap"
)

var (
	cfgFile string
	backend *be.Backend
)

func initConfig() error {
	if cfgFile != "" {
		viper.SetConfigFile(cfgFile)
	} else {
		viper.SetConfigName("infra")
		viper.SetConfigType("yaml")
		viper.AddConfigPath("/etc/strims/")
		viper.AddConfigPath("$HOME/.strims/")
		viper.AddConfigPath(".")
	}

	viper.SetEnvPrefix("STRIMS_")
	viper.AutomaticEnv()

	if err := viper.ReadInConfig(); err != nil {
		return fmt.Errorf("error reading config: %w", err)
	}

	var config be.Config
	if err := viper.Unmarshal(&config, config.DecoderConfigOptions); err != nil {
		return fmt.Errorf("error reading config: %w", err)
	}

	if b, err := be.New(config); err != nil {
		return fmt.Errorf("error starting backend: %w", err)
	} else {
		backend = b
	}

	boil.SetDB(backend.DB)
	return nil
}

func main() {
	flag.StringVar(&cfgFile, "path", "", "path to funding config file")
	flag.Parse()

	initConfig()

	logger, err := zap.NewDevelopment()
	if err != nil {
		log.Fatalln("logger failed:", err)
	}

	srv := &infraServer{
		logger: logger,
	}

	log.Println(srv.Start())
}

type infraServer struct {
	logger       *zap.Logger
	upgrader     websocket.Upgrader
	infraService *infraService
}

func (s *infraServer) Start() error {
	mux := http.NewServeMux()
	mux.HandleFunc("/api", s.handleAPI)

	srv := http.Server{
		Addr:    "0.0.0.0:8085",
		Handler: mux,
	}
	s.logger.Debug("starting server", zap.String("addr", srv.Addr))
	return srv.ListenAndServe()
}

func (s *infraServer) handleAPI(w http.ResponseWriter, r *http.Request) {
	c, err := s.upgrader.Upgrade(w, r, nil)
	if err != nil {
		s.logger.Debug("upgrade failed", zap.Error(err))
		return
	}

	server := rpc.NewServer(s.logger, &rpc.RWDialer{
		Logger:     s.logger,
		ReadWriter: vnic.NewWSReadWriter(c),
	})

	infrav1.RegisterInfraService(server, s.infraService)

	server.Listen(r.Context())
}

type infraService struct {
	logger *zap.Logger
	timer  *time.Timer
}

func (s *infraService) GetHistory(ctx context.Context, req *infrav1.GetHistoryRequest) (*infrav1.GetHistoryResponse, error) {
	return nil, errors.New("unimplemented")
}
