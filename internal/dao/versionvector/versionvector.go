package versionvector

import (
	daov1 "github.com/MemeLabs/strims/pkg/apis/dao/v1"
	"golang.org/x/exp/maps"
)

// New creates a new version vector proceeding from all the supplied values
func New(vs ...*daov1.VersionVector) *daov1.VersionVector {
	d := &daov1.VersionVector{
		Versions: map[uint32]uint64{},
	}
	Update(d, vs...)
	return d
}

func NewSeed(key uint32) *daov1.VersionVector {
	return &daov1.VersionVector{
		Versions: map[uint32]uint64{key: 1},
	}
}

func Update(d *daov1.VersionVector, vs ...*daov1.VersionVector) {
	for _, vv := range vs {
		for i, v := range vv.Versions {
			if d.Versions[i] < v {
				d.Versions[i] = v
			}
		}
	}
}

func Clone(v *daov1.VersionVector) *daov1.VersionVector {
	return &daov1.VersionVector{
		Versions: maps.Clone(v.Versions),
	}
}

// Increment the version corresponding to the replica key
func Increment(v *daov1.VersionVector, key uint32) {
	v.Versions[key]++
}

// Precedes returns true if b descends from a
func Precedes(a, b *daov1.VersionVector) bool {
	for i, v := range a.Versions {
		if b.Versions[i] < v {
			return false
		}
	}
	return true
}

// Conflicts returns true if the versions have both been modified independently
func Conflicts(a, b *daov1.VersionVector) bool {
	return !(Precedes(a, b) || Precedes(b, a))
}
