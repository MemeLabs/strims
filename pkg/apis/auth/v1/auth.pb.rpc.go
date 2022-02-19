package authv1

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterAuthFrontendService ...
func RegisterAuthFrontendService(host rpc.ServiceRegistry, service AuthFrontendService) {
	host.RegisterMethod("strims.auth.v1.AuthFrontend.SignIn", service.SignIn)
	host.RegisterMethod("strims.auth.v1.AuthFrontend.SignUp", service.SignUp)
}

// AuthFrontendService ...
type AuthFrontendService interface {
	SignIn(
		ctx context.Context,
		req *SignInRequest,
	) (*SignInResponse, error)
	SignUp(
		ctx context.Context,
		req *SignUpRequest,
	) (*SignUpResponse, error)
}

// AuthFrontendService ...
type UnimplementedAuthFrontendService struct{}

func (s *UnimplementedAuthFrontendService) SignIn(
	ctx context.Context,
	req *SignInRequest,
) (*SignInResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedAuthFrontendService) SignUp(
	ctx context.Context,
	req *SignUpRequest,
) (*SignUpResponse, error) {
	return nil, rpc.ErrNotImplemented
}

var _ AuthFrontendService = (*UnimplementedAuthFrontendService)(nil)

// AuthFrontendClient ...
type AuthFrontendClient struct {
	client rpc.Caller
}

// NewAuthFrontendClient ...
func NewAuthFrontendClient(client rpc.Caller) *AuthFrontendClient {
	return &AuthFrontendClient{client}
}

// SignIn ...
func (c *AuthFrontendClient) SignIn(
	ctx context.Context,
	req *SignInRequest,
	res *SignInResponse,
) error {
	return c.client.CallUnary(ctx, "strims.auth.v1.AuthFrontend.SignIn", req, res)
}

// SignUp ...
func (c *AuthFrontendClient) SignUp(
	ctx context.Context,
	req *SignUpRequest,
	res *SignUpResponse,
) error {
	return c.client.CallUnary(ctx, "strims.auth.v1.AuthFrontend.SignUp", req, res)
}
