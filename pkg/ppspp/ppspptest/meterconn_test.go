package ppspptest

import (
	"sync"
	"testing"
	"time"

	"github.com/tj/assert"
)

func TestMeterConn(t *testing.T) {
	a, b := NewConnPair()
	am := NewMeterConn(a)
	bm := NewMeterConn(b)

	const n = 1024

	var wg sync.WaitGroup
	wg.Add(2)

	done := time.NewTicker(2 * time.Second)
	go func() {
		defer wg.Done()

		t := time.NewTicker(10 * time.Millisecond)
		b := make([]byte, n)
		for {
			select {
			case <-t.C:
				am.Write(b)
				am.Flush()
			case <-done.C:
				am.Close()
				return
			}
		}
	}()

	go func() {
		defer wg.Done()

		b := make([]byte, n)
		for {
			if _, err := bm.Read(b); err != nil {
				return
			}
		}
	}()

	wg.Wait()

	e := 1

	assert.GreaterOrEqual(t, e, abs(200-int(am.WrittenBytes())/n))
	assert.GreaterOrEqual(t, e, abs(100-int(am.WriteByteRate())/n))
	assert.GreaterOrEqual(t, e, abs(200-int(bm.ReadBytes())/n))
	assert.GreaterOrEqual(t, e, abs(100-int(bm.ReadByteRate())/n))
}

func abs(v int) int {
	if v < 0 {
		return -v
	}
	return v
}
