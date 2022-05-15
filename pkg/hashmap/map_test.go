// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package hashmap

import (
	"encoding/binary"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestSetGetDelete(t *testing.T) {
	m := New[[]byte, uint64](NewByteInterface[[]byte]())
	m.Set([]byte("test"), 1234)

	v, ok := m.Get([]byte("test"))
	assert.EqualValues(t, 1234, v)
	assert.True(t, ok)

	assert.True(t, m.Has([]byte("test")))

	m.Delete([]byte("test"))

	_, ok = m.Get([]byte("test"))
	assert.False(t, ok)
}

func TestIterate(t *testing.T) {
	n := 500
	m := New[[]byte, int](NewByteInterface[[]byte]())
	for i := 0; i < n; i++ {
		b := make([]byte, 4)
		binary.LittleEndian.PutUint32(b, uint32(i))
		m.Set(b, i)
	}

	vs := make([]bool, n)
	for it := m.Iterate(); it.Next(); {
		if vs[it.Value()] {
			t.Errorf("duplicate value in iterator: %d", it.Value())
			t.FailNow()
		}
		vs[it.Value()] = true
	}

	for i := 0; i < n; i++ {
		if !vs[i] {
			t.Errorf("missing value in iterator: %d", i)
			t.FailNow()
		}
	}
}

type testType struct {
	_ [24]byte
}

func BenchmarkMap(b *testing.B) {
	m := New[uint64, testType](NewUint64Interface[uint64]())

	for i := 0; i < b.N; i++ {
		m.Set(uint64(i), testType{})
		if m.Len() > 1000 {
			m.Delete(uint64(i - 1000))
		}
	}
}

func BenchmarkNativeMap(b *testing.B) {
	m := map[uint64]testType{}

	for i := 0; i < b.N; i++ {
		m[uint64(i)] = testType{}
		if len(m) > 1000 {
			delete(m, uint64(i-1000))
		}
	}
}
