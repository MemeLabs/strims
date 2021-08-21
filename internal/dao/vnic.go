package dao

import (
	"errors"

	vnicv1 "github.com/MemeLabs/go-ppspp/pkg/apis/vnic/v1"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
)

// SetVNICConfig ...
func SetVNICConfig(s kv.RWStore, v *vnicv1.Config) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		return tx.Put("vnicIngressConfig", v)
	})
}

// GetVNICConfig ...
func GetVNICConfig(s kv.RWStore) (v *vnicv1.Config, err error) {
	v = &vnicv1.Config{}
	err = s.View(func(tx kv.Tx) error {
		return tx.Get("vnicIngressConfig", v)
	})

	if errors.Is(err, kv.ErrRecordNotFound) {
		v = NewDefaultVNICConfig()
		err = nil
	}
	return
}

// NewDefaultVNICConfig ...
func NewDefaultVNICConfig() *vnicv1.Config {
	return &vnicv1.Config{
		MaxUploadBytesPerSecond: 1 << 40,
	}
}
