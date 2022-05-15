// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build js

package timeutil

import (
	"time"
)

var now Time

func init() {
	now = NewFromTime(time.Now()).Truncate(Precision)
}

// SyncNow ...
func SyncNow(nanos int64) {
	if t := New(nanos).Truncate(Precision); t > now {
		now = t
	}
}

// Now ...
func Now() Time {
	return now
}
