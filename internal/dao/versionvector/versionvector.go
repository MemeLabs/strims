package versionvector

import (
	daov1 "github.com/MemeLabs/strims/pkg/apis/dao/v1"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"golang.org/x/exp/maps"
)

// New creates a new version vector proceeding from all the supplied values
func New(vs ...*daov1.VersionVector) *daov1.VersionVector {
	d := &daov1.VersionVector{
		Value: map[uint32]uint64{},
	}
	Update(d, vs...)
	return d
}

func NewSeed(id uint32) *daov1.VersionVector {
	return &daov1.VersionVector{
		Value:     map[uint32]uint64{id: 1},
		UpdatedAt: timeutil.Now().UnixMilli(),
	}
}

func Update(d *daov1.VersionVector, vs ...*daov1.VersionVector) {
	initValue(d)
	for _, vv := range vs {
		if d.UpdatedAt < vv.UpdatedAt {
			d.UpdatedAt = vv.UpdatedAt
		}

		if vv.Value == nil {
			continue
		}
		for i, v := range vv.Value {
			if d.Value[i] < v {
				d.Value[i] = v
			}
		}
	}
}

func Clone(v *daov1.VersionVector) *daov1.VersionVector {
	return &daov1.VersionVector{
		Value: maps.Clone(v.Value),
	}
}

// Increment the version corresponding to the replica key
func Increment(v *daov1.VersionVector, id uint32) {
	initValue(v)
	v.Value[id]++
	v.UpdatedAt = timeutil.Now().UnixMilli()
}

// Compare returns the integer comparison of two version vectors and whether or
// not they are ordered
func Compare(a, b *daov1.VersionVector) (int, bool) {
	initValue(a)
	initValue(b)
	var lt, gt bool
	for i, v := range b.Value {
		lt = lt || a.Value[i] < v
		gt = gt || a.Value[i] > v
	}
	for i, v := range a.Value {
		lt = lt || b.Value[i] > v
		gt = gt || b.Value[i] < v
	}
	if lt && gt {
		return 0, false
	} else if lt {
		return -1, true
	} else if gt {
		return 1, true
	}
	return 0, true
}

func initValue(v *daov1.VersionVector) {
	if v.Value == nil {
		v.Value = map[uint32]uint64{}
	}
}
