package network

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterBrokerProxyService ...
func RegisterBrokerProxyService(host rpc.ServiceRegistry, service BrokerProxyService) {
	host.RegisterMethod("strims.network.v1.BrokerProxy.Open", service.Open)
	host.RegisterMethod("strims.network.v1.BrokerProxy.SendKeys", service.SendKeys)
	host.RegisterMethod("strims.network.v1.BrokerProxy.ReceiveKeys", service.ReceiveKeys)
	host.RegisterMethod("strims.network.v1.BrokerProxy.Data", service.Data)
	host.RegisterMethod("strims.network.v1.BrokerProxy.Close", service.Close)
}

// BrokerProxyService ...
type BrokerProxyService interface {
	Open(
		ctx context.Context,
		req *BrokerProxyRequest,
	) (<-chan *BrokerProxyEvent, error)
	SendKeys(
		ctx context.Context,
		req *BrokerProxySendKeysRequest,
	) (*BrokerProxySendKeysResponse, error)
	ReceiveKeys(
		ctx context.Context,
		req *BrokerProxyReceiveKeysRequest,
	) (*BrokerProxyReceiveKeysResponse, error)
	Data(
		ctx context.Context,
		req *BrokerProxyDataRequest,
	) (*BrokerProxyDataResponse, error)
	Close(
		ctx context.Context,
		req *BrokerProxyCloseRequest,
	) (*BrokerProxyCloseResponse, error)
}

// BrokerProxyClient ...
type BrokerProxyClient struct {
	client rpc.Caller
}

// NewBrokerProxyClient ...
func NewBrokerProxyClient(client rpc.Caller) *BrokerProxyClient {
	return &BrokerProxyClient{client}
}

// Open ...
func (c *BrokerProxyClient) Open(
	ctx context.Context,
	req *BrokerProxyRequest,
	res chan *BrokerProxyEvent,
) error {
	return c.client.CallStreaming(ctx, "strims.network.v1.BrokerProxy.Open", req, res)
}

// SendKeys ...
func (c *BrokerProxyClient) SendKeys(
	ctx context.Context,
	req *BrokerProxySendKeysRequest,
	res *BrokerProxySendKeysResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.BrokerProxy.SendKeys", req, res)
}

// ReceiveKeys ...
func (c *BrokerProxyClient) ReceiveKeys(
	ctx context.Context,
	req *BrokerProxyReceiveKeysRequest,
	res *BrokerProxyReceiveKeysResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.BrokerProxy.ReceiveKeys", req, res)
}

// Data ...
func (c *BrokerProxyClient) Data(
	ctx context.Context,
	req *BrokerProxyDataRequest,
	res *BrokerProxyDataResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.BrokerProxy.Data", req, res)
}

// Close ...
func (c *BrokerProxyClient) Close(
	ctx context.Context,
	req *BrokerProxyCloseRequest,
	res *BrokerProxyCloseResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.BrokerProxy.Close", req, res)
}
