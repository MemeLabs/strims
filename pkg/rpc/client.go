package rpc

import (
	"context"
	"io"
	"log"
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

	go func() {
		if err := c.readCalls(r); err != nil {
			log.Println(err)
		}
	}()

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

// Done ...
func (c *Client) Done() <-chan struct{} {
	return c.ctx.Done()
}

// Call ...
func (c *Client) Call(ctx context.Context, method string, req proto.Message) error {
	return call(ctx, c.conn, method, req)
}

// CallUnary ...
func (c *Client) CallUnary(ctx context.Context, method string, req, res proto.Message) error {
	r := newCallbackReceiver(c.conn)
	if err := call(ctx, c.conn, method, req, r.CallOption()); err != nil {
		return err
	}
	return r.ReceiveUnary(ctx, res)
}

// CallStreaming ...
func (c *Client) CallStreaming(ctx context.Context, method string, req proto.Message, ch interface{}) error {
	r := newCallbackReceiver(c.conn)
	if err := call(ctx, c.conn, method, req, r.CallOption()); err != nil {
		return err
	}
	go r.ReceiveStream(ctx, ch)
	return nil
}
