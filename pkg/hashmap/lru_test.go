package hashmap

import (
	"encoding/binary"
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"github.com/stretchr/testify/assert"
)

func TestLRUInsertGetDelete(t *testing.T) {
	m := NewLRU[[]byte, uint64](NewByteInterface())
	v, _ := m.GetOrInsert([]byte("test"), 1234)
	assert.EqualValues(t, 1234, v)

	v, ok := m.Get([]byte("test"))
	assert.EqualValues(t, 1234, v)
	assert.True(t, ok)

	assert.True(t, m.Has([]byte("test")))

	m.Delete([]byte("test"))

	_, ok = m.Get([]byte("test"))
	assert.False(t, ok)
}

func TestLRUIterate(t *testing.T) {
	n := 500
	m := NewLRU[[]byte, int](NewByteInterface())
	for i := 0; i < n; i++ {
		b := make([]byte, 4)
		binary.LittleEndian.PutUint32(b, uint32(i))
		m.GetOrInsert(b, i)
	}

	run := func(fn func(vs []bool)) {
		vs := make([]bool, n)
		fn(vs)
		for i := 0; i < n; i++ {
			if !vs[i] {
				t.Errorf("missing value in iterator: %d", i)
				t.FailNow()
			}
		}
	}

	run(func(vs []bool) {
		for it := m.Iterate(); it.Next(); {
			if vs[it.Value()] {
				t.Errorf("duplicate value in iterator: %d", it.Value())
				t.FailNow()
			}
			vs[it.Value()] = true
		}
	})

	run(func(vs []bool) {
		for it := m.IterateTouchedAfter(timeutil.EpochTime); it.Next(); {
			if vs[it.Value()] {
				t.Errorf("duplicate value in iterator: %d", it.Value())
				t.FailNow()
			}
			vs[it.Value()] = true
		}
	})

	run(func(vs []bool) {
		for it := m.IterateTouchedBefore(timeutil.MaxTime); it.Next(); {
			if vs[it.Value()] {
				t.Errorf("duplicate value in iterator: %d", it.Value())
				t.FailNow()
			}
			vs[it.Value()] = true
		}
	})
}

func TestLRUPop(t *testing.T) {
	n := 10
	m := NewLRU[[]byte, int](NewByteInterface())
	for i := 0; i < n; i++ {
		b := make([]byte, 4)
		binary.LittleEndian.PutUint32(b, uint32(i))
		m.GetOrInsert(b, i)
	}

	var nn int
	for {
		if _, ok := m.Pop(timeutil.MaxTime); !ok {
			break
		}
		nn++
	}
	assert.Equal(t, n, nn, "inserted/popped value count mismatch")
	assert.Equal(t, 0, m.Len(), "lru should be empty")
}
