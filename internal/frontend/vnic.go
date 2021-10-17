package frontend

import (
	"context"

	"github.com/MemeLabs/go-ppspp/internal/app"
	vnicv1 "github.com/MemeLabs/go-ppspp/pkg/apis/vnic/v1"
	"github.com/MemeLabs/protobuf/pkg/rpc"
)

func init() {
	RegisterService(func(server *rpc.Server, params *ServiceParams) {
		vnicv1.RegisterVNICFrontendService(server, &vnicFrontendService{
			app: params.App,
		})
	})
}

type vnicFrontendService struct {
	app app.Control
}

func (s *vnicFrontendService) GetConfig(ctx context.Context, r *vnicv1.GetConfigRequest) (*vnicv1.GetConfigResponse, error) {
	config, err := s.app.VNIC().GetConfig()
	if err != nil {
		return nil, err
	}
	return &vnicv1.GetConfigResponse{Config: config}, nil
}

func (s *vnicFrontendService) SetConfig(ctx context.Context, r *vnicv1.SetConfigRequest) (*vnicv1.SetConfigResponse, error) {
	if err := s.app.VNIC().SetConfig(r.Config); err != nil {
		return nil, err
	}
	return &vnicv1.SetConfigResponse{Config: r.Config}, nil
}
