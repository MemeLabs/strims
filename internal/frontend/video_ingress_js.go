//go:build js

package frontend

import (
	"context"

	videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/protobuf/pkg/rpc"
)

func init() {
	RegisterService(func(server *rpc.Server, params ServiceParams) {
		videov1.RegisterVideoIngressService(server, &videoIngressService{})
	})
}

// videoIngressService ...
type videoIngressService struct {
	videov1.UnimplementedVideoIngressService
}

func (s *videoIngressService) IsSupported(ctx context.Context, r *videov1.VideoIngressIsSupportedRequest) (*videov1.VideoIngressIsSupportedResponse, error) {
	return &videov1.VideoIngressIsSupportedResponse{Supported: false}, nil
}
