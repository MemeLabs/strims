// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package dao

import (
	"errors"
	"math"
	"runtime"

	"github.com/MemeLabs/strims/internal/dao/versionvector"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	"github.com/MemeLabs/strims/pkg/kv"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

const (
	_ = iota + profileNS
	profileProfileNS
	profileIDNS
	profileDeviceNS
)

var Profile = NewSingleton[profilev1.Profile](profileProfileNS, nil)

type ProfileIDSingleton struct {
	t *Singleton[profilev1.ProfileID, *profilev1.ProfileID]
}

func (g ProfileIDSingleton) IDGenerator(s kv.RWStore) IDGenerator {
	return IDGeneratorFunc(func() (uint64, error) {
		n, _, err := g.Incr(s, 1)
		return n, err
	})
}

func (g ProfileIDSingleton) Incr(s kv.RWStore, n uint64) (uint64, uint64, error) {
	res, err := g.t.Transform(s, func(v *profilev1.ProfileID) error {
		for v.LastId == v.NextId {
			r := v.NextRange
			if r == nil {
				return errors.New("cannot allocate profile id")
			}
			v.NextId = r.NextId
			v.LastId = r.LastId
			v.NextRange = r.NextRange
		}
		if d := v.LastId - v.NextId; d < n {
			n = d
		}
		v.NextId += n
		return nil
	})
	if err != nil {
		return 0, 0, err
	}
	return res.NextId - n, res.NextId, nil
}

func (g ProfileIDSingleton) FreeCount(s kv.Store) (uint64, error) {
	p, err := g.t.Get(s)
	return countProfileIDs(p), err
}

func (g ProfileIDSingleton) Init(s kv.RWStore, p *profilev1.ProfileID) error {
	return g.t.Set(s, p)
}

func (g ProfileIDSingleton) Pop(s kv.RWStore, n uint64) (*profilev1.ProfileID, error) {
	Logger.Debug("ProfileID.Pop", zap.Uint64("count", n))
	var p *profilev1.ProfileID
	err := s.Update(func(tx kv.RWTx) error {
		for n > 0 {
			nextID, lastID, err := g.Incr(tx, n)
			if err != nil {
				return err
			}
			n -= lastID - nextID
			p = &profilev1.ProfileID{
				NextId:    nextID,
				LastId:    lastID,
				NextRange: p,
			}
		}
		return nil
	})
	if err != nil {
		return nil, err
	}
	return p, nil
}

func (g ProfileIDSingleton) Push(s kv.RWStore, id *profilev1.ProfileID) error {
	Logger.Debug("ProfileID.Push", zap.Uint64("count", countProfileIDs(id)))
	_, err := g.t.Transform(s, func(v *profilev1.ProfileID) error {
		for v.NextRange != nil {
			v = v.NextRange
		}
		v.NextRange = id
		return nil
	})
	return err
}

func countProfileIDs(p *profilev1.ProfileID) uint64 {
	var n uint64
	for ; p != nil; p = p.NextRange {
		n += p.LastId - p.NextId
	}
	return n
}

var ProfileID = ProfileIDSingleton{
	NewSingleton(
		profileIDNS,
		&SingletonOptions[profilev1.ProfileID, *profilev1.ProfileID]{
			DefaultValue: &profilev1.ProfileID{
				NextId: 1,
				LastId: math.MaxUint64,
			},
		},
	),
}

var Devices = NewTable(
	profileDeviceNS,
	&TableOptions[profilev1.Device, *profilev1.Device]{
		ObserveChange: func(m, p *profilev1.Device) proto.Message {
			return &profilev1.DeviceChangeEvent{Device: m}
		},
		ObserveDelete: func(m *profilev1.Device) proto.Message {
			return &profilev1.DeviceDeleteEvent{Device: m}
		},
	},
)

func init() {
	RegisterReplicatedTable(Devices, nil)
}

func initProfileDevice(s kv.RWStore) error {
	device, err := NewDevice(ProfileID.IDGenerator(s), "", runtime.GOOS)
	if err != nil {
		return err
	}
	if err := Devices.Insert(s, device); err != nil {
		return err
	}
	_, err = Profile.Transform(s, func(p *profilev1.Profile) error {
		p.DeviceId = device.Id
		return nil
	})
	return err
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

// NewDevice ...
func NewDevice(g IDGenerator, device, os string) (*profilev1.Device, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	return &profilev1.Device{
		Id:      id,
		Version: versionvector.New(),
		Device:  device,
		Os:      os,
	}, nil
}
