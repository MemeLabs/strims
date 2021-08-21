package dao

import (
	"errors"

	video "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
)

// SetVideoIngressConfig ...
func SetVideoIngressConfig(s kv.RWStore, v *video.VideoIngressConfig) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		return tx.Put("videoIngressConfig", v)
	})
}

// GetVideoIngressConfig ...
func GetVideoIngressConfig(s kv.RWStore) (v *video.VideoIngressConfig, err error) {
	v = &video.VideoIngressConfig{}
	err = s.View(func(tx kv.Tx) error {
		return tx.Get("videoIngressConfig", v)
	})

	if errors.Is(err, kv.ErrRecordNotFound) {
		v = NewDefaultVideoIngressConfig()
		err = nil
	}
	return
}

// NewDefaultVideoIngressConfig ...
func NewDefaultVideoIngressConfig() *video.VideoIngressConfig {
	return &video.VideoIngressConfig{
		ServerAddr: "127.0.0.1:1935",
	}
}
