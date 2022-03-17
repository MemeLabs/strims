package ppspp

import (
	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
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
