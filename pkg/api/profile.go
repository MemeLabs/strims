package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
)

func RegisterProfileService(host *rpc.Host, service ProfileService) {
	host.RegisterService("Profile", service)
}

type ProfileService interface {
	Create(
		ctx context.Context,
		req *pb.CreateProfileRequest,
	) (*pb.CreateProfileResponse, error)
	Load(
		ctx context.Context,
		req *pb.LoadProfileRequest,
	) (*pb.LoadProfileResponse, error)
	Get(
		ctx context.Context,
		req *pb.GetProfileRequest,
	) (*pb.GetProfileResponse, error)
	Update(
		ctx context.Context,
		req *pb.UpdateProfileRequest,
	) (*pb.UpdateProfileResponse, error)
	Delete(
		ctx context.Context,
		req *pb.DeleteProfileRequest,
	) (*pb.DeleteProfileResponse, error)
	List(
		ctx context.Context,
		req *pb.ListProfilesRequest,
	) (*pb.ListProfilesResponse, error)
	LoadSession(
		ctx context.Context,
		req *pb.LoadSessionRequest,
	) (*pb.LoadSessionResponse, error)
}

type ProfileClient struct {
	client *rpc.Client
}

// New ...
func NewProfileClient(client *rpc.Client) *ProfileClient {
	return &ProfileClient{client}
}

// Create ...
func (c *ProfileClient) Create(
	ctx context.Context,
	req *pb.CreateProfileRequest,
	res *pb.CreateProfileResponse,
) error {
	return c.client.CallUnary(ctx, "Profile/Create", req, res)
}

// Load ...
func (c *ProfileClient) Load(
	ctx context.Context,
	req *pb.LoadProfileRequest,
	res *pb.LoadProfileResponse,
) error {
	return c.client.CallUnary(ctx, "Profile/Load", req, res)
}

// Get ...
func (c *ProfileClient) Get(
	ctx context.Context,
	req *pb.GetProfileRequest,
	res *pb.GetProfileResponse,
) error {
	return c.client.CallUnary(ctx, "Profile/Get", req, res)
}

// Update ...
func (c *ProfileClient) Update(
	ctx context.Context,
	req *pb.UpdateProfileRequest,
	res *pb.UpdateProfileResponse,
) error {
	return c.client.CallUnary(ctx, "Profile/Update", req, res)
}

// Delete ...
func (c *ProfileClient) Delete(
	ctx context.Context,
	req *pb.DeleteProfileRequest,
	res *pb.DeleteProfileResponse,
) error {
	return c.client.CallUnary(ctx, "Profile/Delete", req, res)
}

// List ...
func (c *ProfileClient) List(
	ctx context.Context,
	req *pb.ListProfilesRequest,
	res *pb.ListProfilesResponse,
) error {
	return c.client.CallUnary(ctx, "Profile/List", req, res)
}

// LoadSession ...
func (c *ProfileClient) LoadSession(
	ctx context.Context,
	req *pb.LoadSessionRequest,
	res *pb.LoadSessionResponse,
) error {
	return c.client.CallUnary(ctx, "Profile/LoadSession", req, res)
}
