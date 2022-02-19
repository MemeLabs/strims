package networkv1directory

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterDirectoryService ...
func RegisterDirectoryService(host rpc.ServiceRegistry, service DirectoryService) {
	host.RegisterMethod("strims.network.v1.directory.Directory.Publish", service.Publish)
	host.RegisterMethod("strims.network.v1.directory.Directory.Unpublish", service.Unpublish)
	host.RegisterMethod("strims.network.v1.directory.Directory.Join", service.Join)
	host.RegisterMethod("strims.network.v1.directory.Directory.Part", service.Part)
	host.RegisterMethod("strims.network.v1.directory.Directory.Ping", service.Ping)
}

// DirectoryService ...
type DirectoryService interface {
	Publish(
		ctx context.Context,
		req *PublishRequest,
	) (*PublishResponse, error)
	Unpublish(
		ctx context.Context,
		req *UnpublishRequest,
	) (*UnpublishResponse, error)
	Join(
		ctx context.Context,
		req *JoinRequest,
	) (*JoinResponse, error)
	Part(
		ctx context.Context,
		req *PartRequest,
	) (*PartResponse, error)
	Ping(
		ctx context.Context,
		req *PingRequest,
	) (*PingResponse, error)
}

// DirectoryService ...
type UnimplementedDirectoryService struct{}

func (s *UnimplementedDirectoryService) Publish(
	ctx context.Context,
	req *PublishRequest,
) (*PublishResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedDirectoryService) Unpublish(
	ctx context.Context,
	req *UnpublishRequest,
) (*UnpublishResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedDirectoryService) Join(
	ctx context.Context,
	req *JoinRequest,
) (*JoinResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedDirectoryService) Part(
	ctx context.Context,
	req *PartRequest,
) (*PartResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedDirectoryService) Ping(
	ctx context.Context,
	req *PingRequest,
) (*PingResponse, error) {
	return nil, rpc.ErrNotImplemented
}

var _ DirectoryService = (*UnimplementedDirectoryService)(nil)

// DirectoryClient ...
type DirectoryClient struct {
	client rpc.Caller
}

// NewDirectoryClient ...
func NewDirectoryClient(client rpc.Caller) *DirectoryClient {
	return &DirectoryClient{client}
}

// Publish ...
func (c *DirectoryClient) Publish(
	ctx context.Context,
	req *PublishRequest,
	res *PublishResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.directory.Directory.Publish", req, res)
}

// Unpublish ...
func (c *DirectoryClient) Unpublish(
	ctx context.Context,
	req *UnpublishRequest,
	res *UnpublishResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.directory.Directory.Unpublish", req, res)
}

// Join ...
func (c *DirectoryClient) Join(
	ctx context.Context,
	req *JoinRequest,
	res *JoinResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.directory.Directory.Join", req, res)
}

// Part ...
func (c *DirectoryClient) Part(
	ctx context.Context,
	req *PartRequest,
	res *PartResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.directory.Directory.Part", req, res)
}

// Ping ...
func (c *DirectoryClient) Ping(
	ctx context.Context,
	req *PingRequest,
	res *PingResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.directory.Directory.Ping", req, res)
}

// RegisterDirectoryFrontendService ...
func RegisterDirectoryFrontendService(host rpc.ServiceRegistry, service DirectoryFrontendService) {
	host.RegisterMethod("strims.network.v1.directory.DirectoryFrontend.Open", service.Open)
	host.RegisterMethod("strims.network.v1.directory.DirectoryFrontend.Publish", service.Publish)
	host.RegisterMethod("strims.network.v1.directory.DirectoryFrontend.Unpublish", service.Unpublish)
	host.RegisterMethod("strims.network.v1.directory.DirectoryFrontend.Join", service.Join)
	host.RegisterMethod("strims.network.v1.directory.DirectoryFrontend.Part", service.Part)
	host.RegisterMethod("strims.network.v1.directory.DirectoryFrontend.Test", service.Test)
	host.RegisterMethod("strims.network.v1.directory.DirectoryFrontend.GetListingRecord", service.GetListingRecord)
	host.RegisterMethod("strims.network.v1.directory.DirectoryFrontend.ListListingRecords", service.ListListingRecords)
	host.RegisterMethod("strims.network.v1.directory.DirectoryFrontend.UpdateListingRecord", service.UpdateListingRecord)
}

// DirectoryFrontendService ...
type DirectoryFrontendService interface {
	Open(
		ctx context.Context,
		req *FrontendOpenRequest,
	) (<-chan *FrontendOpenResponse, error)
	Publish(
		ctx context.Context,
		req *FrontendPublishRequest,
	) (*FrontendPublishResponse, error)
	Unpublish(
		ctx context.Context,
		req *FrontendUnpublishRequest,
	) (*FrontendUnpublishResponse, error)
	Join(
		ctx context.Context,
		req *FrontendJoinRequest,
	) (*FrontendJoinResponse, error)
	Part(
		ctx context.Context,
		req *FrontendPartRequest,
	) (*FrontendPartResponse, error)
	Test(
		ctx context.Context,
		req *FrontendTestRequest,
	) (*FrontendTestResponse, error)
	GetListingRecord(
		ctx context.Context,
		req *FrontendGetListingRecordRequest,
	) (*FrontendGetListingRecordResponse, error)
	ListListingRecords(
		ctx context.Context,
		req *FrontendListListingRecordsRequest,
	) (*FrontendListListingRecordsResponse, error)
	UpdateListingRecord(
		ctx context.Context,
		req *FrontendUpdateListingRecordRequest,
	) (*FrontendUpdateListingRecordResponse, error)
}

// DirectoryFrontendService ...
type UnimplementedDirectoryFrontendService struct{}

func (s *UnimplementedDirectoryFrontendService) Open(
	ctx context.Context,
	req *FrontendOpenRequest,
) (<-chan *FrontendOpenResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedDirectoryFrontendService) Publish(
	ctx context.Context,
	req *FrontendPublishRequest,
) (*FrontendPublishResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedDirectoryFrontendService) Unpublish(
	ctx context.Context,
	req *FrontendUnpublishRequest,
) (*FrontendUnpublishResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedDirectoryFrontendService) Join(
	ctx context.Context,
	req *FrontendJoinRequest,
) (*FrontendJoinResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedDirectoryFrontendService) Part(
	ctx context.Context,
	req *FrontendPartRequest,
) (*FrontendPartResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedDirectoryFrontendService) Test(
	ctx context.Context,
	req *FrontendTestRequest,
) (*FrontendTestResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedDirectoryFrontendService) GetListingRecord(
	ctx context.Context,
	req *FrontendGetListingRecordRequest,
) (*FrontendGetListingRecordResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedDirectoryFrontendService) ListListingRecords(
	ctx context.Context,
	req *FrontendListListingRecordsRequest,
) (*FrontendListListingRecordsResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedDirectoryFrontendService) UpdateListingRecord(
	ctx context.Context,
	req *FrontendUpdateListingRecordRequest,
) (*FrontendUpdateListingRecordResponse, error) {
	return nil, rpc.ErrNotImplemented
}

var _ DirectoryFrontendService = (*UnimplementedDirectoryFrontendService)(nil)

// DirectoryFrontendClient ...
type DirectoryFrontendClient struct {
	client rpc.Caller
}

// NewDirectoryFrontendClient ...
func NewDirectoryFrontendClient(client rpc.Caller) *DirectoryFrontendClient {
	return &DirectoryFrontendClient{client}
}

// Open ...
func (c *DirectoryFrontendClient) Open(
	ctx context.Context,
	req *FrontendOpenRequest,
	res chan *FrontendOpenResponse,
) error {
	return c.client.CallStreaming(ctx, "strims.network.v1.directory.DirectoryFrontend.Open", req, res)
}

// Publish ...
func (c *DirectoryFrontendClient) Publish(
	ctx context.Context,
	req *FrontendPublishRequest,
	res *FrontendPublishResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.directory.DirectoryFrontend.Publish", req, res)
}

// Unpublish ...
func (c *DirectoryFrontendClient) Unpublish(
	ctx context.Context,
	req *FrontendUnpublishRequest,
	res *FrontendUnpublishResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.directory.DirectoryFrontend.Unpublish", req, res)
}

// Join ...
func (c *DirectoryFrontendClient) Join(
	ctx context.Context,
	req *FrontendJoinRequest,
	res *FrontendJoinResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.directory.DirectoryFrontend.Join", req, res)
}

// Part ...
func (c *DirectoryFrontendClient) Part(
	ctx context.Context,
	req *FrontendPartRequest,
	res *FrontendPartResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.directory.DirectoryFrontend.Part", req, res)
}

// Test ...
func (c *DirectoryFrontendClient) Test(
	ctx context.Context,
	req *FrontendTestRequest,
	res *FrontendTestResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.directory.DirectoryFrontend.Test", req, res)
}

// GetListingRecord ...
func (c *DirectoryFrontendClient) GetListingRecord(
	ctx context.Context,
	req *FrontendGetListingRecordRequest,
	res *FrontendGetListingRecordResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.directory.DirectoryFrontend.GetListingRecord", req, res)
}

// ListListingRecords ...
func (c *DirectoryFrontendClient) ListListingRecords(
	ctx context.Context,
	req *FrontendListListingRecordsRequest,
	res *FrontendListListingRecordsResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.directory.DirectoryFrontend.ListListingRecords", req, res)
}

// UpdateListingRecord ...
func (c *DirectoryFrontendClient) UpdateListingRecord(
	ctx context.Context,
	req *FrontendUpdateListingRecordRequest,
	res *FrontendUpdateListingRecordResponse,
) error {
	return c.client.CallUnary(ctx, "strims.network.v1.directory.DirectoryFrontend.UpdateListingRecord", req, res)
}

// RegisterDirectorySnippetService ...
func RegisterDirectorySnippetService(host rpc.ServiceRegistry, service DirectorySnippetService) {
	host.RegisterMethod("strims.network.v1.directory.DirectorySnippet.Subscribe", service.Subscribe)
}

// DirectorySnippetService ...
type DirectorySnippetService interface {
	Subscribe(
		ctx context.Context,
		req *SnippetSubscribeRequest,
	) (<-chan *SnippetSubscribeResponse, error)
}

// DirectorySnippetService ...
type UnimplementedDirectorySnippetService struct{}

func (s *UnimplementedDirectorySnippetService) Subscribe(
	ctx context.Context,
	req *SnippetSubscribeRequest,
) (<-chan *SnippetSubscribeResponse, error) {
	return nil, rpc.ErrNotImplemented
}

var _ DirectorySnippetService = (*UnimplementedDirectorySnippetService)(nil)

// DirectorySnippetClient ...
type DirectorySnippetClient struct {
	client rpc.Caller
}

// NewDirectorySnippetClient ...
func NewDirectorySnippetClient(client rpc.Caller) *DirectorySnippetClient {
	return &DirectorySnippetClient{client}
}

// Subscribe ...
func (c *DirectorySnippetClient) Subscribe(
	ctx context.Context,
	req *SnippetSubscribeRequest,
	res chan *SnippetSubscribeResponse,
) error {
	return c.client.CallStreaming(ctx, "strims.network.v1.directory.DirectorySnippet.Subscribe", req, res)
}
