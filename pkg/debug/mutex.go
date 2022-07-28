// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package debug

import (
	"fmt"
	"sync"
	"sync/atomic"
	"time"
)

// Mutex ...
type Mutex struct {
	sync.Mutex

	lastMutex        sync.Mutex
	lastAcquiredTime time.Time
	lastAcquired     Tracer
	lastReleasedTime time.Time
	lastReleased     Tracer
	id               string
}

var letters = "abcdefghijklmnopqrstuvwxyz"
var nextLetter = uint64(0)

// String ...
func (d *Mutex) String() string {
	d.lastMutex.Lock()
	defer d.lastMutex.Unlock()

	if d.id == "" {
		d.id = string(letters[atomic.AddUint64(&nextLetter, 1)%uint64(len(letters))])
	}

	return fmt.Sprintf(
		"lock: %s acquired: %s %s released %s %s",
		d.id,
		d.lastAcquiredTime,
		d.lastAcquired.String(),
		d.lastReleasedTime,
		d.lastReleased.String(),
	)
}

// Lock implements sync.Locker
func (d *Mutex) Lock() {
	d.Mutex.Lock()

	d.lastMutex.Lock()
	defer d.lastMutex.Unlock()
	d.lastAcquired.Update(2)
	d.lastAcquiredTime = time.Now()
}

// Unlock implements sync.Locker
func (d *Mutex) Unlock() {
	d.Mutex.Unlock()

	d.lastMutex.Lock()
	defer d.lastMutex.Unlock()
	d.lastReleased.Update(2)
	d.lastReleasedTime = time.Now()
}

func IfLockStalled(l sync.Locker, handlers ...func()) {
	defer IfStalled(handlers...)()
	l.Lock()
}
