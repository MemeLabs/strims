// +build !js

package iotime

import (
	"time"
)

// Load ...
func Load() time.Time {
	return FromTime(time.Now())
}
