//go:build js
// +build js

package frontend

import (
	"context"

	videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/protobuf/pkg/rpc"
)

func init() {
	RegisterService(func(server *rpc.Server, params ServiceParams) {
		videov1.RegisterHLSEgressService(server, &videoHLSEgressService{})
	})
}

// videoHLSEgressService ...
type videoHLSEgressService struct {
	videov1.UnimplementedHLSEgressService
}

// IsSupported ...
func (s *videoHLSEgressService) IsSupported(ctx context.Context, r *videov1.HLSEgressIsSupportedRequest) (*videov1.HLSEgressIsSupportedResponse, error) {
	return &videov1.HLSEgressIsSupportedResponse{Supported: false}, nil
}
