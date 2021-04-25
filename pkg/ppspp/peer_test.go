package ppspp

import (
	"github.com/MemeLabs/go-ppspp/pkg/binmap"
)

type mockPeerWriter struct {
	ID int
}

func (w *mockPeerWriter) Write(maxBytes int) (int, error) { return 0, nil }
func (w *mockPeerWriter) WriteData(maxBytes int, b binmap.Bin, pri peerPriority) (int, error) {
	return 0, nil
}
