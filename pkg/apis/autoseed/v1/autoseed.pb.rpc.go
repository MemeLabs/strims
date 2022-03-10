package autoseedv1

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
)

// RegisterAutoseedFrontendService ...
func RegisterAutoseedFrontendService(host rpc.ServiceRegistry, service AutoseedFrontendService) {
	host.RegisterMethod("strims.autoseed.v1.AutoseedFrontend.GetConfig", service.GetConfig)
	host.RegisterMethod("strims.autoseed.v1.AutoseedFrontend.SetConfig", service.SetConfig)
	host.RegisterMethod("strims.autoseed.v1.AutoseedFrontend.ListRules", service.ListRules)
	host.RegisterMethod("strims.autoseed.v1.AutoseedFrontend.GetRule", service.GetRule)
	host.RegisterMethod("strims.autoseed.v1.AutoseedFrontend.CreateRule", service.CreateRule)
	host.RegisterMethod("strims.autoseed.v1.AutoseedFrontend.UpdateRule", service.UpdateRule)
	host.RegisterMethod("strims.autoseed.v1.AutoseedFrontend.DeleteRule", service.DeleteRule)
}

// AutoseedFrontendService ...
type AutoseedFrontendService interface {
	GetConfig(
		ctx context.Context,
		req *GetConfigRequest,
	) (*GetConfigResponse, error)
	SetConfig(
		ctx context.Context,
		req *SetConfigRequest,
	) (*SetConfigResponse, error)
	ListRules(
		ctx context.Context,
		req *ListRulesRequest,
	) (*ListRulesResponse, error)
	GetRule(
		ctx context.Context,
		req *GetRuleRequest,
	) (*GetRuleResponse, error)
	CreateRule(
		ctx context.Context,
		req *CreateRuleRequest,
	) (*CreateRuleResponse, error)
	UpdateRule(
		ctx context.Context,
		req *UpdateRuleRequest,
	) (*UpdateRuleResponse, error)
	DeleteRule(
		ctx context.Context,
		req *DeleteRuleRequest,
	) (*DeleteRuleResponse, error)
}

// AutoseedFrontendService ...
type UnimplementedAutoseedFrontendService struct{}

func (s *UnimplementedAutoseedFrontendService) GetConfig(
	ctx context.Context,
	req *GetConfigRequest,
) (*GetConfigResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedAutoseedFrontendService) SetConfig(
	ctx context.Context,
	req *SetConfigRequest,
) (*SetConfigResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedAutoseedFrontendService) ListRules(
	ctx context.Context,
	req *ListRulesRequest,
) (*ListRulesResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedAutoseedFrontendService) GetRule(
	ctx context.Context,
	req *GetRuleRequest,
) (*GetRuleResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedAutoseedFrontendService) CreateRule(
	ctx context.Context,
	req *CreateRuleRequest,
) (*CreateRuleResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedAutoseedFrontendService) UpdateRule(
	ctx context.Context,
	req *UpdateRuleRequest,
) (*UpdateRuleResponse, error) {
	return nil, rpc.ErrNotImplemented
}

func (s *UnimplementedAutoseedFrontendService) DeleteRule(
	ctx context.Context,
	req *DeleteRuleRequest,
) (*DeleteRuleResponse, error) {
	return nil, rpc.ErrNotImplemented
}

var _ AutoseedFrontendService = (*UnimplementedAutoseedFrontendService)(nil)

// AutoseedFrontendClient ...
type AutoseedFrontendClient struct {
	client rpc.Caller
}

// NewAutoseedFrontendClient ...
func NewAutoseedFrontendClient(client rpc.Caller) *AutoseedFrontendClient {
	return &AutoseedFrontendClient{client}
}

// GetConfig ...
func (c *AutoseedFrontendClient) GetConfig(
	ctx context.Context,
	req *GetConfigRequest,
	res *GetConfigResponse,
) error {
	return c.client.CallUnary(ctx, "strims.autoseed.v1.AutoseedFrontend.GetConfig", req, res)
}

// SetConfig ...
func (c *AutoseedFrontendClient) SetConfig(
	ctx context.Context,
	req *SetConfigRequest,
	res *SetConfigResponse,
) error {
	return c.client.CallUnary(ctx, "strims.autoseed.v1.AutoseedFrontend.SetConfig", req, res)
}

// ListRules ...
func (c *AutoseedFrontendClient) ListRules(
	ctx context.Context,
	req *ListRulesRequest,
	res *ListRulesResponse,
) error {
	return c.client.CallUnary(ctx, "strims.autoseed.v1.AutoseedFrontend.ListRules", req, res)
}

// GetRule ...
func (c *AutoseedFrontendClient) GetRule(
	ctx context.Context,
	req *GetRuleRequest,
	res *GetRuleResponse,
) error {
	return c.client.CallUnary(ctx, "strims.autoseed.v1.AutoseedFrontend.GetRule", req, res)
}

// CreateRule ...
func (c *AutoseedFrontendClient) CreateRule(
	ctx context.Context,
	req *CreateRuleRequest,
	res *CreateRuleResponse,
) error {
	return c.client.CallUnary(ctx, "strims.autoseed.v1.AutoseedFrontend.CreateRule", req, res)
}

// UpdateRule ...
func (c *AutoseedFrontendClient) UpdateRule(
	ctx context.Context,
	req *UpdateRuleRequest,
	res *UpdateRuleResponse,
) error {
	return c.client.CallUnary(ctx, "strims.autoseed.v1.AutoseedFrontend.UpdateRule", req, res)
}

// DeleteRule ...
func (c *AutoseedFrontendClient) DeleteRule(
	ctx context.Context,
	req *DeleteRuleRequest,
	res *DeleteRuleResponse,
) error {
	return c.client.CallUnary(ctx, "strims.autoseed.v1.AutoseedFrontend.DeleteRule", req, res)
}
