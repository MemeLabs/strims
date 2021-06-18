package ppspp

import (
	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
)

type mockPeerWriter struct {
	ID int
}

func (w *mockPeerWriter) Write(maxBytes int) (int, error) { return 0, nil }
func (w *mockPeerWriter) WriteData(maxBytes int, b binmap.Bin, t timeutil.Time, pri peerPriority) (int, error) {
	return 0, nil
}
