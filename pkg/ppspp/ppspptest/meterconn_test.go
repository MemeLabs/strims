package ppspptest

import (
	"sync"
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/mathutil"
	"github.com/stretchr/testify/assert"
)

func TestMeterConn(t *testing.T) {
	a, b := NewConnPair()
	am := NewMeterConn(a)
	bm := NewMeterConn(b)

	const n = 10240

	var wg sync.WaitGroup
	wg.Add(2)

	done := time.NewTicker(2 * time.Second)
	tick := time.NewTicker(100 * time.Millisecond)

	go func() {
		defer wg.Done()

		b := make([]byte, n)
		for {
			select {
			case <-tick.C:
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

	assert.GreaterOrEqual(t, e, mathutil.Abs(20-int(am.WrittenBytes())/n))
	assert.GreaterOrEqual(t, e, mathutil.Abs(10-int(am.WriteByteRate())/n))
	assert.GreaterOrEqual(t, e, mathutil.Abs(20-int(bm.ReadBytes())/n))
	assert.GreaterOrEqual(t, e, mathutil.Abs(10-int(bm.ReadByteRate())/n))
}
