//go:build !js
// +build !js

package frontend

import (
	"context"

	"github.com/MemeLabs/go-ppspp/internal/app"
	videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/protobuf/pkg/rpc"
	"go.uber.org/zap"
)

func init() {
	RegisterService(func(server *rpc.Server, params ServiceParams) {
		videov1.RegisterHLSEgressService(server, &videoHLSEgressService{
			app:    params.App,
			logger: params.Logger,
		})
	})
}

// videoHLSEgressService ...
type videoHLSEgressService struct {
	app    app.Control
	logger *zap.Logger
}

// IsSupported ...
func (s *videoHLSEgressService) IsSupported(ctx context.Context, r *videov1.HLSEgressIsSupportedRequest) (*videov1.HLSEgressIsSupportedResponse, error) {
	return &videov1.HLSEgressIsSupportedResponse{Supported: true}, nil
}

// OpenStream ...
func (s *videoHLSEgressService) OpenStream(ctx context.Context, r *videov1.HLSEgressOpenStreamRequest) (*videov1.HLSEgressOpenStreamResponse, error) {
	uri, err := s.app.VideoEgress().OpenHLSStream(r.SwarmUri, r.NetworkKeys)
	if err != nil {
		s.logger.Debug("opening stream failed", zap.Error(err))
		return nil, err
	}

	return &videov1.HLSEgressOpenStreamResponse{
		PlaylistUrl: uri,
	}, nil
}

// CloseStream ...
func (s *videoHLSEgressService) CloseStream(ctx context.Context, r *videov1.HLSEgressCloseStreamRequest) (*videov1.HLSEgressCloseStreamResponse, error) {
	return nil, rpc.ErrNotImplemented
}
