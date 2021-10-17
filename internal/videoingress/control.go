package videoingress

import (
	"context"

	videov1 "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
)

type Control interface {
	Run(ctx context.Context)
	GetIngressConfig() (*videov1.VideoIngressConfig, error)
	SetIngressConfig(config *videov1.VideoIngressConfig) error
}
