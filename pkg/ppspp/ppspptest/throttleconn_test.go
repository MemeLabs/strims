package ppspptest

import (
	"io"
	"io/ioutil"
	"sync"
	"testing"
	"time"

	"github.com/tj/assert"
)

func TestConnThrottle(t *testing.T) {
	a, b := NewConnPair()
	a = NewThrottleConn(a, NewConnThrottle(10*Kbps, 10*Kbps))
	b = NewThrottleConn(b, NewConnThrottle(10*Kbps, 10*Kbps))

	start := time.Now()
	var wg sync.WaitGroup
	wg.Add(2)

	go func() {
		defer wg.Done()
		d := make([]byte, 10*Kbps)
		for {
			if _, err := a.Write(d); err != nil {
				return
			}
			if err := a.Flush(); err != nil {
				return
			}
		}
	}()

	go func() {
		defer wg.Done()
		if _, err := io.CopyN(ioutil.Discard, b, 30*Kbps); err != nil {
			t.Error(err)
		}
		b.Close()
	}()

	wg.Wait()
	assert.LessOrEqual(t, int64(3*time.Second), int64(time.Since(start)), "completed too quickly")
}
