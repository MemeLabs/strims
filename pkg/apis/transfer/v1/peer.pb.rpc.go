package transfer

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterTransferPeerService ...
func RegisterTransferPeerService(host rpc.ServiceRegistry, service TransferPeerService) {
	host.RegisterMethod("strims.transfer.v1.TransferPeer.Announce", service.Announce)
	host.RegisterMethod("strims.transfer.v1.TransferPeer.Close", service.Close)
}

// TransferPeerService ...
type TransferPeerService interface {
	Announce(
		ctx context.Context,
		req *TransferPeerAnnounceRequest,
	) (*TransferPeerAnnounceResponse, error)
	Close(
		ctx context.Context,
		req *TransferPeerCloseRequest,
	) (*TransferPeerCloseResponse, error)
}

// TransferPeerService ...
type UnimplementedTransferPeerService struct{}

func (s *UnimplementedTransferPeerService) Announce(
	ctx context.Context,
	req *TransferPeerAnnounceRequest,
) (*TransferPeerAnnounceResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedTransferPeerService) Close(
	ctx context.Context,
	req *TransferPeerCloseRequest,
) (*TransferPeerCloseResponse, error) {
	return nil, rpc.ErrNotImplemented
}

var _ TransferPeerService = (*UnimplementedTransferPeerService)(nil)

// TransferPeerClient ...
type TransferPeerClient struct {
	client rpc.Caller
}

// NewTransferPeerClient ...
func NewTransferPeerClient(client rpc.Caller) *TransferPeerClient {
	return &TransferPeerClient{client}
}

// Announce ...
func (c *TransferPeerClient) Announce(
	ctx context.Context,
	req *TransferPeerAnnounceRequest,
	res *TransferPeerAnnounceResponse,
) error {
	return c.client.CallUnary(ctx, "strims.transfer.v1.TransferPeer.Announce", req, res)
}

// Close ...
func (c *TransferPeerClient) Close(
	ctx context.Context,
	req *TransferPeerCloseRequest,
	res *TransferPeerCloseResponse,
) error {
	return c.client.CallUnary(ctx, "strims.transfer.v1.TransferPeer.Close", req, res)
}
