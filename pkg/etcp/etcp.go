package etcp

// implements Elastic-TCP congestion control algorithm
// see: https://ieeexplore.ieee.org/document/8642512

import (
	"log"
	"math"
	"time"
)

// NewControl ...
func NewControl() *Control {
	return &Control{
		rttMax:  0,
		rttCur:  0,
		rttBase: 0x7fffffff,
		cwnd:    2,
	}
}

// Control ...
type Control struct {
	rttMax  float64
	rttCur  float64
	rttBase float64
	cwnd    float64

	// lastDataLoss time.Time
}

// OnAck ...
func (c *Control) OnAck(rtt time.Duration) {
	// data loss...

	c.rttCur = float64(rtt)
	if c.rttCur < c.rttBase {
		c.rttBase = c.rttCur
	}
	if c.rttCur > c.rttMax {
		c.rttMax = c.rttCur
	}
	wwf := math.Sqrt(c.rttMax / c.rttCur * c.cwnd)

	// debug.LogEveryN(100, ">>>>", wwf)

	c.cwnd += wwf / c.cwnd
	// log.Println("wwf: ", wwf)
}

// OnDataLoss ...
func (c *Control) OnDataLoss() {
	// now := timeutil.Now()
	// if now.Sub(c.lastDataLoss) > time.Duration(c.rttCur) {
	c.cwnd /= 2
	if c.cwnd < 2 {
		c.cwnd = 2
	}
	// c.lastDataLoss = now
	// }
}

// CWND ...
func (c *Control) CWND() int {
	return int(c.cwnd)
}

// DebugPrint ...
func (c *Control) DebugPrint() {
	log.Printf(
		"rttMax: %-7.2f rttCur: %-7.2f rttBase: %-7.2f cwnd: %-7.2f",
		c.rttMax,
		c.rttCur,
		c.rttBase,
		c.cwnd,
	)
}
