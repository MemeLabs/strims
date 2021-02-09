package etcp

// implements Elastic-TCP congestion control algorithm
// see: https://ieeexplore.ieee.org/document/8642512

import (
	"testing"
	"time"
)

func TestETCP(t *testing.T) {
	rtt := 250 * time.Millisecond
	rttt := time.NewTicker(rtt)

	c := NewControl()
	epoch := time.Now()
	for t := range rttt.C {
		c.OnAck(t.Sub(epoch))
		c.DebugPrint()
		epoch = t
	}
}
