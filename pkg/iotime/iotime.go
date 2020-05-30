package iotime

import "time"

// FromTime ...
func FromTime(t time.Time) time.Time {
	return t.UTC().Truncate(time.Millisecond)
}
