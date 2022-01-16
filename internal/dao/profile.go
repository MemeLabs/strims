package dao

import (
	"strconv"

	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
)

const profileKeyPrefix = "profile:"

func prefixProfileKey(id uint64) string {
	return profileKeyPrefix + strconv.FormatUint(id, 10)
}

// GetProfile ...
func GetProfile(s kv.Store) (v *profilev1.Profile, err error) {
	v = &profilev1.Profile{}
	err = s.View(func(tx kv.Tx) error {
		return tx.Get("profile", v)
	})
	return
}

// NewProfile ...
func NewProfile(name string) (p *profilev1.Profile, err error) {
	p = &profilev1.Profile{
		Name: name,
	}

	p.Key, err = GenerateKey()
	if err != nil {
		return nil, err
	}

	p.Id, err = GenerateSnowflake()
	if err != nil {
		return nil, err
	}

	return p, nil
}
