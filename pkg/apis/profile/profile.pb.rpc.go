package profile

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/api"
)

// RegisterProfileServiceService ...
func RegisterProfileServiceService(host api.ServiceRegistry, service ProfileServiceService) {
	host.RegisterMethod(".strims.profile.ProfileService.Create", service.Create)
	host.RegisterMethod(".strims.profile.ProfileService.Load", service.Load)
	host.RegisterMethod(".strims.profile.ProfileService.Get", service.Get)
	host.RegisterMethod(".strims.profile.ProfileService.Update", service.Update)
	host.RegisterMethod(".strims.profile.ProfileService.Delete", service.Delete)
	host.RegisterMethod(".strims.profile.ProfileService.List", service.List)
	host.RegisterMethod(".strims.profile.ProfileService.LoadSession", service.LoadSession)
}

// ProfileServiceService ...
type ProfileServiceService interface {
	Create(
		ctx context.Context,
		req *CreateProfileRequest,
	) (*CreateProfileResponse, error)
	Load(
		ctx context.Context,
		req *LoadProfileRequest,
	) (*LoadProfileResponse, error)
	Get(
		ctx context.Context,
		req *GetProfileRequest,
	) (*GetProfileResponse, error)
	Update(
		ctx context.Context,
		req *UpdateProfileRequest,
	) (*UpdateProfileResponse, error)
	Delete(
		ctx context.Context,
		req *DeleteProfileRequest,
	) (*DeleteProfileResponse, error)
	List(
		ctx context.Context,
		req *ListProfilesRequest,
	) (*ListProfilesResponse, error)
	LoadSession(
		ctx context.Context,
		req *LoadSessionRequest,
	) (*LoadSessionResponse, error)
}

// ProfileServiceClient ...
type ProfileServiceClient struct {
	client api.Caller
}

// NewProfileServiceClient ...
func NewProfileServiceClient(client api.Caller) *ProfileServiceClient {
	return &ProfileServiceClient{client}
}

// Create ...
func (c *ProfileServiceClient) Create(
	ctx context.Context,
	req *CreateProfileRequest,
	res *CreateProfileResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.profile.ProfileService.Create", req, res)
}

// Load ...
func (c *ProfileServiceClient) Load(
	ctx context.Context,
	req *LoadProfileRequest,
	res *LoadProfileResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.profile.ProfileService.Load", req, res)
}

// Get ...
func (c *ProfileServiceClient) Get(
	ctx context.Context,
	req *GetProfileRequest,
	res *GetProfileResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.profile.ProfileService.Get", req, res)
}

// Update ...
func (c *ProfileServiceClient) Update(
	ctx context.Context,
	req *UpdateProfileRequest,
	res *UpdateProfileResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.profile.ProfileService.Update", req, res)
}

// Delete ...
func (c *ProfileServiceClient) Delete(
	ctx context.Context,
	req *DeleteProfileRequest,
	res *DeleteProfileResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.profile.ProfileService.Delete", req, res)
}

// List ...
func (c *ProfileServiceClient) List(
	ctx context.Context,
	req *ListProfilesRequest,
	res *ListProfilesResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.profile.ProfileService.List", req, res)
}

// LoadSession ...
func (c *ProfileServiceClient) LoadSession(
	ctx context.Context,
	req *LoadSessionRequest,
	res *LoadSessionResponse,
) error {
	return c.client.CallUnary(ctx, ".strims.profile.ProfileService.LoadSession", req, res)
}
