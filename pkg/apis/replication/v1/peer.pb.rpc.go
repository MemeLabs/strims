package replicationv1

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterReplicationPeerService ...
func RegisterReplicationPeerService(host rpc.ServiceRegistry, service ReplicationPeerService) {
	host.RegisterMethod("strims.replication.v1.ReplicationPeer.Open", service.Open)
	host.RegisterMethod("strims.replication.v1.ReplicationPeer.SendEvents", service.SendEvents)
	host.RegisterMethod("strims.replication.v1.ReplicationPeer.AllocateProfileIDs", service.AllocateProfileIDs)
}

// ReplicationPeerService ...
type ReplicationPeerService interface {
	Open(
		ctx context.Context,
		req *PeerOpenRequest,
	) (*PeerOpenResponse, error)
	SendEvents(
		ctx context.Context,
		req *PeerSendEventsRequest,
	) (*PeerSendEventsResponse, error)
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

func (s *UnimplementedReplicationPeerService) SendEvents(
	ctx context.Context,
	req *PeerSendEventsRequest,
) (*PeerSendEventsResponse, error) {
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

// SendEvents ...
func (c *ReplicationPeerClient) SendEvents(
	ctx context.Context,
	req *PeerSendEventsRequest,
	res *PeerSendEventsResponse,
) error {
	return c.client.CallUnary(ctx, "strims.replication.v1.ReplicationPeer.SendEvents", req, res)
}

// AllocateProfileIDs ...
func (c *ReplicationPeerClient) AllocateProfileIDs(
	ctx context.Context,
	req *PeerAllocateProfileIDsRequest,
	res *PeerAllocateProfileIDsResponse,
) error {
	return c.client.CallUnary(ctx, "strims.replication.v1.ReplicationPeer.AllocateProfileIDs", req, res)
}
