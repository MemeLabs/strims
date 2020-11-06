package rpc

import (
	"context"

	"github.com/golang/protobuf/proto"
)

// NewClient ...
func NewClient(dialer Dialer) (*Client, error) {
	transport, err := dialer.Dial(context.Background(), &clientDispatcher{})
	if err != nil {
		return nil, err
	}

	return &Client{
		transport: transport,
	}, nil
}

// Client ...
type Client struct {
	transport Transport
}

// CallUnary ...
func (c *Client) CallUnary(ctx context.Context, method string, req, res proto.Message) error {
	call, err := NewCallOut(ctx, method, req)
	if err != nil {
		return err
	}

	return c.transport.Call(call, func() error {
		return call.ReadResponse(res)
	})
}

// CallStreaming ...
func (c *Client) CallStreaming(ctx context.Context, method string, req proto.Message, res interface{}) error {
	call, err := NewCallOut(ctx, method, req)
	if err != nil {
		return err
	}

	return c.transport.Call(call, func() error {
		return call.ReadResponseStream(res)
	})
}

type clientDispatcher struct{}

func (c *clientDispatcher) Dispatch(call *CallIn) {
	if call.Method() != callbackMethod {
		return
	}

	parent, ok := call.Parent().(*CallOut)
	if !ok {
		return
	}

	parent.AssignResponse(call)
}
