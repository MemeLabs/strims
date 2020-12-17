package dao

import (
	"errors"

	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// SetVideoIngressConfig ...
func SetVideoIngressConfig(s kv.RWStore, v *pb.VideoIngressConfig) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		return tx.Put("videoIngressConfig", v)
	})
}

// GetVideoIngressConfig ...
func GetVideoIngressConfig(s kv.RWStore) (v *pb.VideoIngressConfig, err error) {
	v = &pb.VideoIngressConfig{}
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
func NewDefaultVideoIngressConfig() *pb.VideoIngressConfig {
	return &pb.VideoIngressConfig{
		ServerAddr: "127.0.0.1:1935",
	}
}
