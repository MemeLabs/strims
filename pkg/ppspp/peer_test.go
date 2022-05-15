// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ppspp

import (
	"github.com/MemeLabs/strims/pkg/binmap"
	"github.com/MemeLabs/strims/pkg/timeutil"
)

type mockPeerWriter struct {
	peerTaskRunnerQueueTicket
	ID int
}

func (w *mockPeerWriter) WriteHandshake() error { return nil }
func (w *mockPeerWriter) Write() (int, error)   { return 0, nil }
func (w *mockPeerWriter) WriteData(b binmap.Bin, t timeutil.Time, pri peerPriority) (int, error) {
	return 0, nil
}
