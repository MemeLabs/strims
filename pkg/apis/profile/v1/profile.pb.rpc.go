package profilev1

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterProfileFrontendService ...
func RegisterProfileFrontendService(host rpc.ServiceRegistry, service ProfileFrontendService) {
	host.RegisterMethod("strims.profile.v1.ProfileFrontend.Get", service.Get)
	host.RegisterMethod("strims.profile.v1.ProfileFrontend.Update", service.Update)
	host.RegisterMethod("strims.profile.v1.ProfileFrontend.DeleteDevice", service.DeleteDevice)
	host.RegisterMethod("strims.profile.v1.ProfileFrontend.GetDevice", service.GetDevice)
	host.RegisterMethod("strims.profile.v1.ProfileFrontend.ListDevices", service.ListDevices)
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
	DeleteDevice(
		ctx context.Context,
		req *DeleteDeviceRequest,
	) (*DeleteDeviceResponse, error)
	GetDevice(
		ctx context.Context,
		req *GetDeviceRequest,
	) (*GetDeviceResponse, error)
	ListDevices(
		ctx context.Context,
		req *ListDevicesRequest,
	) (*ListDevicesResponse, error)
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

func (s *UnimplementedProfileFrontendService) DeleteDevice(
	ctx context.Context,
	req *DeleteDeviceRequest,
) (*DeleteDeviceResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedProfileFrontendService) GetDevice(
	ctx context.Context,
	req *GetDeviceRequest,
) (*GetDeviceResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedProfileFrontendService) ListDevices(
	ctx context.Context,
	req *ListDevicesRequest,
) (*ListDevicesResponse, error) {
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

// DeleteDevice ...
func (c *ProfileFrontendClient) DeleteDevice(
	ctx context.Context,
	req *DeleteDeviceRequest,
	res *DeleteDeviceResponse,
) error {
	return c.client.CallUnary(ctx, "strims.profile.v1.ProfileFrontend.DeleteDevice", req, res)
}

// GetDevice ...
func (c *ProfileFrontendClient) GetDevice(
	ctx context.Context,
	req *GetDeviceRequest,
	res *GetDeviceResponse,
) error {
	return c.client.CallUnary(ctx, "strims.profile.v1.ProfileFrontend.GetDevice", req, res)
}

// ListDevices ...
func (c *ProfileFrontendClient) ListDevices(
	ctx context.Context,
	req *ListDevicesRequest,
	res *ListDevicesResponse,
) error {
	return c.client.CallUnary(ctx, "strims.profile.v1.ProfileFrontend.ListDevices", req, res)
}
