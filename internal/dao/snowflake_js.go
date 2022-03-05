//go:build js

package dao

import (
	"sync/atomic"
	"time"
)

var nextSnowflakeID uint64

// GenerateSnowflake generate a 64 bit locally unique id
func GenerateSnowflake() (uint64, error) {
	seconds := uint64(time.Since(time.Date(2020, 0, 0, 0, 0, 0, 0, time.UTC)) / time.Second)
	sequence := atomic.AddUint64(&nextSnowflakeID, 1) << 32
	return seconds | sequence, nil
}
