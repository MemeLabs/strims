package profilev1

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterProfileFrontendService ...
func RegisterProfileFrontendService(host rpc.ServiceRegistry, service ProfileFrontendService) {
	host.RegisterMethod("strims.profile.v1.ProfileFrontend.Get", service.Get)
	host.RegisterMethod("strims.profile.v1.ProfileFrontend.Update", service.Update)
}

// ProfileFrontendService ...
type ProfileFrontendService interface {
	Get(
		ctx context.Context,
		req *GetProfileRequest,
	) (*GetProfileResponse, error)
	Update(
		ctx context.Context,
		req *UpdateProfileRequest,
	) (*UpdateProfileResponse, error)
}

// ProfileFrontendService ...
type UnimplementedProfileFrontendService struct{}

func (s *UnimplementedProfileFrontendService) Get(
	ctx context.Context,
	req *GetProfileRequest,
) (*GetProfileResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedProfileFrontendService) Update(
	ctx context.Context,
	req *UpdateProfileRequest,
) (*UpdateProfileResponse, error) {
	return nil, rpc.ErrNotImplemented
}

var _ ProfileFrontendService = (*UnimplementedProfileFrontendService)(nil)

// ProfileFrontendClient ...
type ProfileFrontendClient struct {
	client rpc.Caller
}

// NewProfileFrontendClient ...
func NewProfileFrontendClient(client rpc.Caller) *ProfileFrontendClient {
	return &ProfileFrontendClient{client}
}

// Get ...
func (c *ProfileFrontendClient) Get(
	ctx context.Context,
	req *GetProfileRequest,
	res *GetProfileResponse,
) error {
	return c.client.CallUnary(ctx, "strims.profile.v1.ProfileFrontend.Get", req, res)
}

// Update ...
func (c *ProfileFrontendClient) Update(
	ctx context.Context,
	req *UpdateProfileRequest,
	res *UpdateProfileResponse,
) error {
	return c.client.CallUnary(ctx, "strims.profile.v1.ProfileFrontend.Update", req, res)
}
