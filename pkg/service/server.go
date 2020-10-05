package service

import (
	"context"
	"io"

	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"go.uber.org/zap"
)

// Options ...
type Options struct {
	Store      kv.BlobStore
	Logger     *zap.Logger
	VPNOptions []vpn.HostOption
}

// New ...
func New(options Options) (*Server, error) {
	metadata, err := dao.NewMetadataStore(options.Store)
	if err != nil {
		return nil, err
	}

	host := rpc.NewHost(options.Logger)
	api.RegisterFrontendRPCService(host, &Frontend{
		logger:     options.Logger,
		store:      options.Store,
		metadata:   metadata,
		vpnOptions: options.VPNOptions,
	})
	return &Server{host}, nil
}

// Server ...
type Server struct {
	host *rpc.Host
}

// Listen ...
func (s *Server) Listen(ctx context.Context, rw io.ReadWriter) {
	ctx = contextWithSession(ctx, newSession())
	s.host.Listen(ctx, rw)
}
