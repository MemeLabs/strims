// +build js

package iotime

import (
	"time"
)

var timestamp int64

// Store ...
func Store(t int64) {
	timestamp = t
}

// Load ...
func Load() time.Time {
	return time.Unix(timestamp/1000, (timestamp%1000)*1000000)
}
