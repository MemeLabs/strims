// +build !js

package timeutil

import (
	"time"
)

// Now ...
func Now() Time {
	return NewFromTime(time.Now()).Truncate(time.Millisecond)
}
