package service

import (
	"context"
	"io"

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

	svc := &Frontend{
		logger:     options.Logger,
		store:      options.Store,
		metadata:   metadata,
		vpnOptions: options.VPNOptions,
	}
	return &Server{rpc.NewHost(options.Logger, svc)}, nil
}

// Server ...
type Server struct {
	host *rpc.Host
}

// Listen ...
func (s *Server) Listen(ctx context.Context, rw io.ReadWriter) error {
	ctx = contextWithSession(ctx, newSession())
	return s.host.Listen(ctx, rw)
}
