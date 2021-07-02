package vnic

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterVNICFrontendService ...
func RegisterVNICFrontendService(host rpc.ServiceRegistry, service VNICFrontendService) {
	host.RegisterMethod("strims.vnic.v1.VNICFrontend.GetConfig", service.GetConfig)
	host.RegisterMethod("strims.vnic.v1.VNICFrontend.SetConfig", service.SetConfig)
}

// VNICFrontendService ...
type VNICFrontendService interface {
	GetConfig(
		ctx context.Context,
		req *GetConfigRequest,
	) (*GetConfigResponse, error)
	SetConfig(
		ctx context.Context,
		req *SetConfigRequest,
	) (*SetConfigResponse, error)
}

// VNICFrontendClient ...
type VNICFrontendClient struct {
	client rpc.Caller
}

// NewVNICFrontendClient ...
func NewVNICFrontendClient(client rpc.Caller) *VNICFrontendClient {
	return &VNICFrontendClient{client}
}

// GetConfig ...
func (c *VNICFrontendClient) GetConfig(
	ctx context.Context,
	req *GetConfigRequest,
	res *GetConfigResponse,
) error {
	return c.client.CallUnary(ctx, "strims.vnic.v1.VNICFrontend.GetConfig", req, res)
}

// SetConfig ...
func (c *VNICFrontendClient) SetConfig(
	ctx context.Context,
	req *SetConfigRequest,
	res *SetConfigResponse,
) error {
	return c.client.CallUnary(ctx, "strims.vnic.v1.VNICFrontend.SetConfig", req, res)
}
