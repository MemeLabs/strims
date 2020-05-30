package rpc

import (
	"context"
	"io"
	"sync"

	"github.com/golang/protobuf/proto"
)

// NewClient ...
func NewClient(w io.Writer, r io.Reader) *Client {
	ctx, cancel := context.WithCancel(context.Background())
	c := &Client{
		ctx:    ctx,
		cancel: cancel,
		conn:   &conn{w: w},
	}

	go c.readCalls(r)

	return c
}

// Client ...
type Client struct {
	ctx       context.Context
	cancel    context.CancelFunc
	closeOnce sync.Once
	conn      *conn
}

func (c *Client) readCalls(r io.Reader) error {
	for {
		m, err := readCall(r)
		if err != nil {
			return err
		}

		if m.Method == callbackMethod {
			go handleCallback(c.conn, m)
		}

		if err := c.ctx.Err(); err != nil {
			return err
		}
	}
}

// Close ...
func (c *Client) Close() {
	c.closeOnce.Do(func() { c.cancel() })
}

// Call ...
func (c *Client) Call(ctx context.Context, method string, req proto.Message) error {
	_, err := call(ctx, c.conn, method, req)
	return err
}

// CallUnary ...
func (c *Client) CallUnary(ctx context.Context, method string, req, res proto.Message) error {
	l, err := call(ctx, c.conn, method, req)
	if err != nil {
		return err
	}
	return expectOne(ctx, c.conn, l, res)
}

// CallStreaming ...
func (c *Client) CallStreaming(ctx context.Context, method string, req proto.Message, ch interface{}) error {
	l, err := call(ctx, c.conn, method, req)
	if err != nil {
		return err
	}
	return expectMany(ctx, c.conn, l, ch)
}
