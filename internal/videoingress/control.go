package videoingress

import (
	videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
)

type Control interface {
	Run()
	GetIngressConfig() (*videov1.VideoIngressConfig, error)
	SetIngressConfig(config *videov1.VideoIngressConfig) error
}
