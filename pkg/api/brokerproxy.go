package api

import (
	"context"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// RegisterBrokerProxyService ...
func RegisterBrokerProxyService(host ServiceRegistry, service BrokerProxyService) {
	host.RegisterMethod("BrokerProxy/Open", service.Open)
	host.RegisterMethod("BrokerProxy/SendKeys", service.SendKeys)
	host.RegisterMethod("BrokerProxy/ReceiveKeys", service.ReceiveKeys)
	host.RegisterMethod("BrokerProxy/Data", service.Data)
	host.RegisterMethod("BrokerProxy/Close", service.Close)
}

// BrokerProxyService ...
type BrokerProxyService interface {
	Open(
		ctx context.Context,
		req *pb.BrokerProxyRequest,
	) (<-chan *pb.BrokerProxyEvent, error)
	SendKeys(
		ctx context.Context,
		req *pb.BrokerProxySendKeysRequest,
	) (*pb.BrokerProxySendKeysResponse, error)
	ReceiveKeys(
		ctx context.Context,
		req *pb.BrokerProxyReceiveKeysRequest,
	) (*pb.BrokerProxyReceiveKeysResponse, error)
	Data(
		ctx context.Context,
		req *pb.BrokerProxyDataRequest,
	) (*pb.BrokerProxyDataResponse, error)
	Close(
		ctx context.Context,
		req *pb.BrokerProxyCloseRequest,
	) (*pb.BrokerProxyCloseResponse, error)
}

// BrokerProxyClient ...
type BrokerProxyClient struct {
	client Caller
}

// NewBrokerProxyClient ...
func NewBrokerProxyClient(client Caller) *BrokerProxyClient {
	return &BrokerProxyClient{client}
}

// Open ...
func (c *BrokerProxyClient) Open(
	ctx context.Context,
	req *pb.BrokerProxyRequest,
	res chan *pb.BrokerProxyEvent,
) error {
	return c.client.CallStreaming(ctx, "BrokerProxy/Open", req, res)
}

// SendKeys ...
func (c *BrokerProxyClient) SendKeys(
	ctx context.Context,
	req *pb.BrokerProxySendKeysRequest,
	res *pb.BrokerProxySendKeysResponse,
) error {
	return c.client.CallUnary(ctx, "BrokerProxy/SendKeys", req, res)
}

// ReceiveKeys ...
func (c *BrokerProxyClient) ReceiveKeys(
	ctx context.Context,
	req *pb.BrokerProxyReceiveKeysRequest,
	res *pb.BrokerProxyReceiveKeysResponse,
) error {
	return c.client.CallUnary(ctx, "BrokerProxy/ReceiveKeys", req, res)
}

// Data ...
func (c *BrokerProxyClient) Data(
	ctx context.Context,
	req *pb.BrokerProxyDataRequest,
	res *pb.BrokerProxyDataResponse,
) error {
	return c.client.CallUnary(ctx, "BrokerProxy/Data", req, res)
}

// Close ...
func (c *BrokerProxyClient) Close(
	ctx context.Context,
	req *pb.BrokerProxyCloseRequest,
	res *pb.BrokerProxyCloseResponse,
) error {
	return c.client.CallUnary(ctx, "BrokerProxy/Close", req, res)
}
