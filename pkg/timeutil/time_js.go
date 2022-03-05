//go:build js

package timeutil

import (
	"time"
)

var now int64

func init() {
	now = time.Now().UnixNano()
}

// SyncNow ...
func SyncNow(t int64) {
	if t > now {
		now = t
	}
}

// Now ...
func Now() Time {
	return New(now)
}
