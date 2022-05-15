// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package servicemanager

import (
	"testing"
	"time"
)

func TestStopperEmptyState(t *testing.T) {
	var s Stopper

	deadline := time.NewTicker(10 * time.Millisecond)
	defer deadline.Stop()

	select {
	case <-s.Stop():
	case <-deadline.C:
		t.Error("channel from Stop() on uninitialized Stopper should not block")
	}
}
