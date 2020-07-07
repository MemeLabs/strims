package ppspp

import (
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/tj/assert"
)

func TestBinTimeoutQueue(t *testing.T) {
	q := newBinTimeoutQueue(16)
	ts := time.Now()
	for i := 0; i < 30; i++ {
		q.Push(binmap.Bin(i), ts)
	}

	var n = 0
	for it := q.IterateUntil(ts.Add(time.Second)); it.Next(); {
		assert.EqualValues(t, n, it.Bin(), "value mismatch")
		n++
	}
	assert.Equal(t, 30, n, "value count mismatch")
}
