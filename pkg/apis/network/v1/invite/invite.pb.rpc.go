package networkv1invite

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterInviteLinkService ...
func RegisterInviteLinkService(host rpc.ServiceRegistry, service InviteLinkService) {
	host.RegisterMethod("strims.network.v1.invite.InviteLink.GetInvitation", service.GetInvitation)
}

// InviteLinkService ...
type InviteLinkService interface {
	GetInvitation(
		ctx context.Context,
		req *GetInvitationRequest,
	) (*GetInvitationResponse, error)
}

// InviteLinkService ...
type UnimplementedInviteLinkService struct{}

func (s *UnimplementedInviteLinkService) GetInvitation(
	ctx context.Context,
	req *GetInvitationRequest,
) (*GetInvitationResponse, error) {
	return nil, rpc.ErrNotImplemented
}

var _ InviteLinkService = (*UnimplementedInviteLinkService)(nil)

// InviteLinkClient ...
type InviteLinkClient struct {
	client rpc.Caller
}

// NewInviteLinkClient ...
func NewInviteLinkClient(client rpc.Caller) *InviteLinkClient {
	return &InviteLinkClient{client}
}

// GetInvitation ...
func (c *InviteLinkClient) GetInvitation(
	ctx context.Context,
	req *GetInvitationRequest,
	res *GetInvitationResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.invite.InviteLink.GetInvitation", req, res)
}
