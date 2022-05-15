// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build !js

package timeutil

import (
	"time"
)

// Now ...
func Now() Time {
	return NewFromTime(time.Now()).Truncate(Precision)
}
