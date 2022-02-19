package ppspp

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterCapConnService ...
func RegisterCapConnService(host rpc.ServiceRegistry, service CapConnService) {
	host.RegisterMethod("strims.devtools.v1.ppspp.CapConn.WatchLogs", service.WatchLogs)
	host.RegisterMethod("strims.devtools.v1.ppspp.CapConn.LoadLog", service.LoadLog)
}

// CapConnService ...
type CapConnService interface {
	WatchLogs(
		ctx context.Context,
		req *CapConnWatchLogsRequest,
	) (<-chan *CapConnWatchLogsResponse, error)
	LoadLog(
		ctx context.Context,
		req *CapConnLoadLogRequest,
	) (*CapConnLoadLogResponse, error)
}

// CapConnService ...
type UnimplementedCapConnService struct{}

func (s *UnimplementedCapConnService) WatchLogs(
	ctx context.Context,
	req *CapConnWatchLogsRequest,
) (<-chan *CapConnWatchLogsResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedCapConnService) LoadLog(
	ctx context.Context,
	req *CapConnLoadLogRequest,
) (*CapConnLoadLogResponse, error) {
	return nil, rpc.ErrNotImplemented
}

var _ CapConnService = (*UnimplementedCapConnService)(nil)

// CapConnClient ...
type CapConnClient struct {
	client rpc.Caller
}

// NewCapConnClient ...
func NewCapConnClient(client rpc.Caller) *CapConnClient {
	return &CapConnClient{client}
}

// WatchLogs ...
func (c *CapConnClient) WatchLogs(
	ctx context.Context,
	req *CapConnWatchLogsRequest,
	res chan *CapConnWatchLogsResponse,
) error {
	return c.client.CallStreaming(ctx, "strims.devtools.v1.ppspp.CapConn.WatchLogs", req, res)
}

// LoadLog ...
func (c *CapConnClient) LoadLog(
	ctx context.Context,
	req *CapConnLoadLogRequest,
	res *CapConnLoadLogResponse,
) error {
	return c.client.CallUnary(ctx, "strims.devtools.v1.ppspp.CapConn.LoadLog", req, res)
}
