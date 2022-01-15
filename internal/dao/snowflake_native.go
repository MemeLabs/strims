//go:build !js
// +build !js

package dao

import (
	"time"

	"github.com/sony/sonyflake"
)

var snowflake = sonyflake.NewSonyflake(sonyflake.Settings{
	StartTime: time.Date(2021, 1, 1, 0, 0, 0, 0, time.UTC),
})

// GenerateSnowflake generate a 63 bit probably globally unique id
func GenerateSnowflake() (uint64, error) {
	return snowflake.NextID()
}
