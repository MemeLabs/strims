// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package dao

import (
	autoseedv1 "github.com/MemeLabs/strims/pkg/apis/autoseed/v1"
	"google.golang.org/protobuf/proto"
)

const (
	_ = iota + autoseedNS
	autoseedConfigNS
	autoseedRuleNS
)

var AutoseedConfig = NewSingleton(
	autoseedConfigNS,
	&SingletonOptions[autoseedv1.Config, *autoseedv1.Config]{
		DefaultValue: &autoseedv1.Config{},
		ObserveChange: func(m, p *autoseedv1.Config) proto.Message {
			return &autoseedv1.ConfigChangeEvent{Config: m}
		},
	},
)

var AutoseedRules = NewTable(
	autoseedRuleNS,
	&TableOptions[autoseedv1.Rule, *autoseedv1.Rule]{
		ObserveChange: func(m, p *autoseedv1.Rule) proto.Message {
			return &autoseedv1.RuleChangeEvent{Rule: m}
		},
		ObserveDelete: func(m *autoseedv1.Rule) proto.Message {
			return &autoseedv1.RuleDeleteEvent{Rule: m}
		},
	},
)

// NewAutoseedRule ...
func NewAutoseedRule(g IDGenerator, label string, networkKey, swarmID, salt []byte) (*autoseedv1.Rule, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	return &autoseedv1.Rule{
		Id:         id,
		Label:      label,
		NetworkKey: networkKey,
		SwarmId:    swarmID,
		Salt:       salt,
	}, nil
}
