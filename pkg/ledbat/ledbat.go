// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ledbat

import (
	"math"
	"time"

	"github.com/MemeLabs/strims/pkg/stats"
	"github.com/MemeLabs/strims/pkg/timeutil"
)

const (
	// rfc6817
	target          = int(100 * time.Millisecond)
	allowedIncrease = 4
	gain            = 1
	baseHistory     = 4
	currentDelays   = 10
	// initCWND        = 2
	// gotta go fast
	initCWND = 32
	minCWND  = 2
	mss      = 1024

	// rfc6298
	coefG = 100000000.0 // 100ms
	coefK = 4

	// jacobson, v. "congestion avoidance and control"
	// doi: 10.1145/52325.52356
	coefAlpha = 0.125
	coefBeta  = 0.25

	maxDelaySample = time.Duration(math.MaxInt64)
)

func filter(durations []time.Duration) (min time.Duration) {
	min = maxDelaySample
	for _, d := range durations {
		if d < min {
			min = d
		}
	}
	return
}

// New ...
func New() *Controller {
	l := &Controller{
		baseDelays:    make([]time.Duration, baseHistory),
		currentDelays: make([]time.Duration, currentDelays),

		cwnd: initCWND * mss,
		cto:  time.Second,

		lastDataLoss: timeutil.NilTime,
		lastAckTime:  timeutil.MaxTime,
		rttMean:      stats.NewEMA(coefAlpha),
		rttVar:       stats.NewEMA(coefBeta),
		debug:        false,
	}

	for i := range l.baseDelays {
		l.baseDelays[i] = maxDelaySample
	}
	for i := range l.currentDelays {
		l.currentDelays[i] = maxDelaySample
	}

	return l
}

// Controller ...
type Controller struct {
	baseDelays        []time.Duration
	currentDelays     []time.Duration
	baseDelayIndex    int
	currentDelayIndex int
	lastRollover      timeutil.Time

	flightSize int
	cwnd       int

	// congestion timeout
	cto time.Duration

	lastDataLoss timeutil.Time
	lastAckTime  timeutil.Time
	rttMean      stats.EMA
	rttVar       stats.EMA

	ackSize int

	debug bool
}

// account for scheduled competing flow
func (l *Controller) HackTest(size int) {
	l.cwnd -= size
	if min := minCWND * mss; min > l.cwnd {
		l.cwnd = min
	}
}

// Debug ...
func (l *Controller) Debug() bool {
	return l.debug
}

// StartDebugging ...
func (l *Controller) StartDebugging() {
	l.debug = true
}

// CWND ...
func (l *Controller) CWND() int {
	return l.cwnd
}

// CTO ...
func (l *Controller) CTO() time.Duration {
	return l.cto
}

// FlightSize ...
func (l *Controller) FlightSize() int {
	return l.flightSize
}

// RTTMean ...
func (l *Controller) RTTMean() time.Duration {
	return time.Duration(l.rttMean.Value())
}

// RTTVar ...
func (l *Controller) RTTVar() time.Duration {
	return time.Duration(l.rttVar.Value())
}

// AddSent ...
func (l *Controller) AddSent(size int) {
	l.flightSize += size
}

// AddDelaySample ...
func (l *Controller) AddDelaySample(d time.Duration, size int) {
	l.updateCurrentDelay(d)
	l.updateBaseDelay(d)

	l.ackSize += size

	l.lastAckTime = timeutil.Now()
}

func (l *Controller) updateCurrentDelay(d time.Duration) {
	l.currentDelayIndex++
	if l.currentDelayIndex == currentDelays {
		l.currentDelayIndex = 0
	}
	l.currentDelays[l.currentDelayIndex] = d
}

func (l *Controller) updateBaseDelay(d time.Duration) {
	now := timeutil.Now().Truncate(time.Minute)
	if now != l.lastRollover {
		l.lastRollover = now

		l.baseDelayIndex++
		if l.baseDelayIndex == baseHistory {
			l.baseDelayIndex = 0
		}

		l.baseDelays[l.baseDelayIndex] = d
		return
	}

	if d < l.baseDelays[l.baseDelayIndex] {
		l.baseDelays[l.baseDelayIndex] = d
	}
}

// DigestDelaySamples ...
func (l *Controller) DigestDelaySamples() {
	// if no acks have been received in cto (heavy congestion) reset cwnd
	// and adjust cto
	if l.flightSize > 0 && timeutil.Now().Sub(l.lastAckTime) > l.cto {
		l.cwnd = minCWND * mss
		l.cto = 2 * l.cto
		if l.cto > time.Second {
			l.cto = time.Second
		}
	}

	if l.ackSize == 0 {
		return
	}

	queuingDelay := int(filter(l.currentDelays) - filter(l.baseDelays))
	l.cwnd += ((target - queuingDelay) * gain * l.ackSize * mss) / l.cwnd / target
	// if max := l.flightSize + allowedIncrease*mss; max < l.cwnd {
	// 	l.cwnd = max
	// }
	if min := minCWND * mss; min > l.cwnd {
		l.cwnd = min
	}

	l.flightSize -= l.ackSize
	if l.flightSize < 0 {
		l.flightSize = 0
	}
	l.ackSize = 0
}

// AddRTTSample ...
func (l *Controller) AddRTTSample(rtt time.Duration) {
	rttNanos := float64(rtt)
	if l.rttMean.Value() == 0 {
		l.rttMean.Set(rttNanos)
		l.rttVar.Set(rttNanos / 2)
	} else {
		l.rttVar.Update(math.Abs(l.rttMean.Value() - rttNanos))
		l.rttMean.Update(rttNanos)
	}

	ctoNanos := l.rttMean.Value() + math.Max(coefG, coefK*l.rttVar.Value())
	l.cto = time.Duration(ctoNanos)
}

// AddDataLoss ...
func (l *Controller) AddDataLoss(size int, retransmitting bool) {
	if !retransmitting {
		l.flightSize -= size
		if l.flightSize < 0 {
			l.flightSize = 0
		}
	}

	timeout := time.Duration(l.rttMean.Value())
	// if timeout < time.Second*2 {
	// 	timeout = time.Second * 2
	// }
	now := timeutil.Now()
	if !l.lastDataLoss.IsNil() && now.Sub(l.lastDataLoss) < timeout {
		return
	}
	l.lastDataLoss = now

	cwnd := l.cwnd / 2
	if min := minCWND * mss; min > cwnd {
		cwnd = min
	}
	if cwnd < l.cwnd {
		l.cwnd = cwnd
	}
}
