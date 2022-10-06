package replicationv1

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterReplicationPeerService ...
func RegisterReplicationPeerService(host rpc.ServiceRegistry, service ReplicationPeerService) {
	host.RegisterMethod("strims.replication.v1.ReplicationPeer.Open", service.Open)
	host.RegisterMethod("strims.replication.v1.ReplicationPeer.Bootstrap", service.Bootstrap)
	host.RegisterMethod("strims.replication.v1.ReplicationPeer.Sync", service.Sync)
	host.RegisterMethod("strims.replication.v1.ReplicationPeer.AllocateProfileIDs", service.AllocateProfileIDs)
}

// ReplicationPeerService ...
type ReplicationPeerService interface {
	Open(
		ctx context.Context,
		req *PeerOpenRequest,
	) (*PeerOpenResponse, error)
	Bootstrap(
		ctx context.Context,
		req *PeerBootstrapRequest,
	) (*PeerBootstrapResponse, error)
	Sync(
		ctx context.Context,
		req *PeerSyncRequest,
	) (*PeerSyncResponse, error)
	AllocateProfileIDs(
		ctx context.Context,
		req *PeerAllocateProfileIDsRequest,
	) (*PeerAllocateProfileIDsResponse, error)
}

// ReplicationPeerService ...
type UnimplementedReplicationPeerService struct{}

func (s *UnimplementedReplicationPeerService) Open(
	ctx context.Context,
	req *PeerOpenRequest,
) (*PeerOpenResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedReplicationPeerService) Bootstrap(
	ctx context.Context,
	req *PeerBootstrapRequest,
) (*PeerBootstrapResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedReplicationPeerService) Sync(
	ctx context.Context,
	req *PeerSyncRequest,
) (*PeerSyncResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedReplicationPeerService) AllocateProfileIDs(
	ctx context.Context,
	req *PeerAllocateProfileIDsRequest,
) (*PeerAllocateProfileIDsResponse, error) {
	return nil, rpc.ErrNotImplemented
}

var _ ReplicationPeerService = (*UnimplementedReplicationPeerService)(nil)

// ReplicationPeerClient ...
type ReplicationPeerClient struct {
	client rpc.Caller
}

// NewReplicationPeerClient ...
func NewReplicationPeerClient(client rpc.Caller) *ReplicationPeerClient {
	return &ReplicationPeerClient{client}
}

// Open ...
func (c *ReplicationPeerClient) Open(
	ctx context.Context,
	req *PeerOpenRequest,
	res *PeerOpenResponse,
) error {
	return c.client.CallUnary(ctx, "strims.replication.v1.ReplicationPeer.Open", req, res)
}

// Bootstrap ...
func (c *ReplicationPeerClient) Bootstrap(
	ctx context.Context,
	req *PeerBootstrapRequest,
	res *PeerBootstrapResponse,
) error {
	return c.client.CallUnary(ctx, "strims.replication.v1.ReplicationPeer.Bootstrap", req, res)
}

// Sync ...
func (c *ReplicationPeerClient) Sync(
	ctx context.Context,
	req *PeerSyncRequest,
	res *PeerSyncResponse,
) error {
	return c.client.CallUnary(ctx, "strims.replication.v1.ReplicationPeer.Sync", req, res)
}

// AllocateProfileIDs ...
func (c *ReplicationPeerClient) AllocateProfileIDs(
	ctx context.Context,
	req *PeerAllocateProfileIDsRequest,
	res *PeerAllocateProfileIDsResponse,
) error {
	return c.client.CallUnary(ctx, "strims.replication.v1.ReplicationPeer.AllocateProfileIDs", req, res)
}
