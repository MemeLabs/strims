// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package debug

import "time"

func StartTimer() *Timer {
	return &Timer{time.Now()}
}

type Timer struct {
	start time.Time
}

func (t *Timer) Start() {
	t.start = time.Now()
}

func (t Timer) Elapsed() time.Duration {
	return time.Since(t.start)
}
