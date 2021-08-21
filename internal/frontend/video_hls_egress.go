//go:build !js
// +build !js

package frontend

import (
	"context"

	control "github.com/MemeLabs/go-ppspp/internal"
	videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/protobuf/pkg/rpc"
)

func init() {
	RegisterService(func(server *rpc.Server, params *ServiceParams) {
		videov1.RegisterHLSEgressService(server, &videoHLSEgressService{
			app: params.App,
		})
	})
}

// videoHLSEgressService ...
type videoHLSEgressService struct {
	app control.AppControl
}

// IsSupported ...
func (s *videoHLSEgressService) IsSupported(ctx context.Context, r *videov1.HLSEgressIsSupportedRequest) (*videov1.HLSEgressIsSupportedResponse, error) {
	return &videov1.HLSEgressIsSupportedResponse{Supported: true}, nil
}

// OpenStream ...
func (s *videoHLSEgressService) OpenStream(ctx context.Context, r *videov1.HLSEgressOpenStreamRequest) (*videov1.HLSEgressOpenStreamResponse, error) {
	return nil, rpc.ErrNotImplemented
}

// CloseStream ...
func (s *videoHLSEgressService) CloseStream(ctx context.Context, r *videov1.HLSEgressCloseStreamRequest) (*videov1.HLSEgressCloseStreamResponse, error) {

	return nil, rpc.ErrNotImplemented
}
