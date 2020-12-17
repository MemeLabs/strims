package directory

import (
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/iotime"
	"github.com/petar/GoLLRB/llrb"
	"github.com/stretchr/testify/assert"
)

type testItem struct {
	key []byte
}

func (u *testItem) Key() []byte {
	return u.key
}

func (u *testItem) Less(o llrb.Item) bool {
	return keyerLess(u, o)
}

func TestLRUSetGet(t *testing.T) {
	var l lru

	a := &testItem{[]byte("a")}
	b := &testItem{[]byte("b")}
	c := &testItem{[]byte("c")}

	l.GetOrInsert(a)
	l.GetOrInsert(b)
	l.GetOrInsert(c)

	assert.Same(t, a, l.GetOrInsert(a))
	assert.Same(t, a, l.GetOrInsert(&lruKey{[]byte("a")}))
	assert.Same(t, a, l.Get(a))
	assert.Same(t, a, l.Get(&lruKey{[]byte("a")}))
}

func TestLRUPeekRecentlyTouched(t *testing.T) {
	var l lru

	start := iotime.Load()

	l.GetOrInsert(&testItem{[]byte("a")})
	l.GetOrInsert(&testItem{[]byte("b")})
	l.GetOrInsert(&testItem{[]byte("c")})

	keys := []string{}
	for it := l.PeekRecentlyTouched(start); it.Next(); {
		keys = append(keys, string(it.Value().(*testItem).key))
	}
	assert.Equal(t, []string{"c", "b", "a"}, keys)

	time.Sleep(time.Millisecond)
	start = iotime.Load()

	l.GetOrInsert(&testItem{[]byte("b")})
	l.GetOrInsert(&testItem{[]byte("d")})

	keys = []string{}
	for it := l.PeekRecentlyTouched(start); it.Next(); {
		keys = append(keys, string(it.Value().(*testItem).key))
	}
	assert.Equal(t, []string{"d", "b"}, keys)
}
