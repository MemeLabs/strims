package hashmap

import (
	"encoding/binary"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestSetGetDelete(t *testing.T) {
	m := New[[]byte, uint64](NewByteInterface())
	m.Set([]byte("test"), 1234)

	v, ok := m.Get([]byte("test"))
	assert.EqualValues(t, 1234, v)
	assert.True(t, ok)

	m.Delete([]byte("test"))

	_, ok = m.Get([]byte("test"))
	assert.False(t, ok)
}

func TestIterate(t *testing.T) {
	n := 500
	m := New[[]byte, int](NewByteInterface())
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
