package dao

import (
	vnicv1 "github.com/MemeLabs/go-ppspp/pkg/apis/vnic/v1"
)

const (
	_ = iota + vnicNS
	vnicConfigNS
)

var VNICConfig = NewSingleton(
	vnicConfigNS,
	&SingletonOptions[vnicv1.Config]{
		DefaultValue: &vnicv1.Config{
			MaxUploadBytesPerSecond: 1 << 40,
		},
	},
)
