package replicationv1

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterReplicationFrontendService ...
func RegisterReplicationFrontendService(host rpc.ServiceRegistry, service ReplicationFrontendService) {
	host.RegisterMethod("strims.replication.v1.ReplicationFrontend.CreatePairingToken", service.CreatePairingToken)
}

// ReplicationFrontendService ...
type ReplicationFrontendService interface {
	CreatePairingToken(
		ctx context.Context,
		req *CreatePairingTokenRequest,
	) (*CreatePairingTokenResponse, error)
}

// ReplicationFrontendService ...
type UnimplementedReplicationFrontendService struct{}

func (s *UnimplementedReplicationFrontendService) CreatePairingToken(
	ctx context.Context,
	req *CreatePairingTokenRequest,
) (*CreatePairingTokenResponse, error) {
	return nil, rpc.ErrNotImplemented
}

var _ ReplicationFrontendService = (*UnimplementedReplicationFrontendService)(nil)

// ReplicationFrontendClient ...
type ReplicationFrontendClient struct {
	client rpc.Caller
}

// NewReplicationFrontendClient ...
func NewReplicationFrontendClient(client rpc.Caller) *ReplicationFrontendClient {
	return &ReplicationFrontendClient{client}
}

// CreatePairingToken ...
func (c *ReplicationFrontendClient) CreatePairingToken(
	ctx context.Context,
	req *CreatePairingTokenRequest,
	res *CreatePairingTokenResponse,
) error {
	return c.client.CallUnary(ctx, "strims.replication.v1.ReplicationFrontend.CreatePairingToken", req, res)
}
