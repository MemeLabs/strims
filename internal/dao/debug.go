// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package dao

import (
	debugv1 "github.com/MemeLabs/strims/pkg/apis/debug/v1"
	"google.golang.org/protobuf/proto"
)

const (
	_ = iota + debugNS
	debugConfigNS
)

var DebugConfig = NewSingleton(
	debugConfigNS,
	&SingletonOptions[debugv1.Config, *debugv1.Config]{
		DefaultValue: &debugv1.Config{},
		ObserveChange: func(m, p *debugv1.Config) proto.Message {
			return &debugv1.ConfigChangeEvent{Config: m}
		},
	},
)
