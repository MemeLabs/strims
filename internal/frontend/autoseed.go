// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package frontend

import (
	"context"

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/internal/dao"
	autoseedv1 "github.com/MemeLabs/strims/pkg/apis/autoseed/v1"
)

func init() {
	RegisterService(func(server *rpc.Server, params ServiceParams) {
		svc := &autoseedService{
			store: params.Store,
		}
		autoseedv1.RegisterAutoseedFrontendService(server, svc)
	})
}

// autoseedService ...
type autoseedService struct {
	autoseedv1.UnimplementedAutoseedFrontendService
	store *dao.ProfileStore
}

func (s *autoseedService) GetConfig(ctx context.Context, r *autoseedv1.GetConfigRequest) (*autoseedv1.GetConfigResponse, error) {
	config, err := dao.AutoseedConfig.Get(s.store)
	if err != nil {
		return nil, err
	}
	return &autoseedv1.GetConfigResponse{Config: config}, nil
}

func (s *autoseedService) SetConfig(ctx context.Context, r *autoseedv1.SetConfigRequest) (*autoseedv1.SetConfigResponse, error) {
	if err := dao.AutoseedConfig.Set(s.store, r.Config); err != nil {
		return nil, err
	}
	return &autoseedv1.SetConfigResponse{Config: r.Config}, nil
}

func (s *autoseedService) ListRules(ctx context.Context, r *autoseedv1.ListRulesRequest) (*autoseedv1.ListRulesResponse, error) {
	rules, err := dao.AutoseedRules.GetAll(s.store)
	if err != nil {
		return nil, err
	}
	return &autoseedv1.ListRulesResponse{Rules: rules}, nil
}

func (s *autoseedService) GetRule(ctx context.Context, r *autoseedv1.GetRuleRequest) (*autoseedv1.GetRuleResponse, error) {
	rule, err := dao.AutoseedRules.Get(s.store, r.Id)
	if err != nil {
		return nil, err
	}
	return &autoseedv1.GetRuleResponse{Rule: rule}, nil
}

func (s *autoseedService) CreateRule(ctx context.Context, r *autoseedv1.CreateRuleRequest) (*autoseedv1.CreateRuleResponse, error) {
	rule, err := dao.NewAutoseedRule(s.store, r.Rule.Label, r.Rule.NetworkKey, r.Rule.SwarmId, r.Rule.Salt)
	if err != nil {
		return nil, err
	}

	err = dao.AutoseedRules.Insert(s.store, rule)
	if err != nil {
		return nil, err
	}
	return &autoseedv1.CreateRuleResponse{Rule: rule}, nil
}

func (s *autoseedService) UpdateRule(ctx context.Context, r *autoseedv1.UpdateRuleRequest) (*autoseedv1.UpdateRuleResponse, error) {
	rule, err := dao.AutoseedRules.Transform(s.store, r.Id, func(p *autoseedv1.Rule) error {
		p.Label = r.Rule.Label
		p.NetworkKey = r.Rule.NetworkKey
		p.SwarmId = r.Rule.SwarmId
		p.Salt = r.Rule.Salt
		return nil
	})
	if err != nil {
		return nil, err
	}
	return &autoseedv1.UpdateRuleResponse{Rule: rule}, nil
}

func (s *autoseedService) DeleteRule(ctx context.Context, r *autoseedv1.DeleteRuleRequest) (*autoseedv1.DeleteRuleResponse, error) {
	err := dao.AutoseedRules.Delete(s.store, r.Id)
	if err != nil {
		return nil, err
	}
	return &autoseedv1.DeleteRuleResponse{}, nil
}
