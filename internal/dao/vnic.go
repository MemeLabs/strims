// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package dao

import (
	vnicv1 "github.com/MemeLabs/strims/pkg/apis/vnic/v1"
	"google.golang.org/protobuf/proto"
)

const (
	_ = iota + vnicNS
	vnicConfigNS
)

var VNICConfig = NewSingleton(
	vnicConfigNS,
	&SingletonOptions[vnicv1.Config, *vnicv1.Config]{
		DefaultValue: &vnicv1.Config{
			MaxUploadBytesPerSecond: 1 << 40,
			MaxPeers:                25,
		},
		ObserveChange: func(m, p *vnicv1.Config) proto.Message {
			return &vnicv1.ConfigChangeEvent{Config: m}
		},
	},
)
