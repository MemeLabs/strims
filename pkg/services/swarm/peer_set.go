package swarm

import (
	"context"
	"crypto/rand"
	"encoding/json"
	"fmt"
	"io"
	"path"
	"runtime"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
)

type peerSetItem vpn.PeerIndexHost

func (t *peerSetItem) Less(oi llrb.Item) bool {
	o, ok := oi.(*peerSetItem)
	if !ok {
		return true
	}
	if t.Port == o.Port {
		return t.HostID.Less(o.HostID)
	}
	return t.Port < o.Port
}

func newPeerSet() *peerSet {
	return &peerSet{
		values: llrb.New(),
	}
}

type peerSet struct {
	values *llrb.LLRB
}

func (s *peerSet) LoadFrom(ctx context.Context, idx vpn.PeerIndex, key, salt []byte) error {
	hosts, err := idx.Search(ctx, key, salt)
	if err != nil {
		return err
	}

	for h := range hosts {
		s.Insert(h)
	}
	return nil
}

func (s *peerSet) Insert(h *vpn.PeerIndexHost) {
	item := (*peerSetItem)(h)
	old, ok := s.values.Get(item).(*peerSetItem)
	if !ok || old.Timestamp.Before(h.Timestamp) {
		s.values.ReplaceOrInsert(item)
	}
}

func (s *peerSet) Slice() []*vpn.PeerIndexHost {
	vs := make([]*vpn.PeerIndexHost, 0, s.values.Len())
	s.values.AscendLessThan(llrb.Inf(1), func(t llrb.Item) bool {
		vs = append(vs, (*vpn.PeerIndexHost)(t.(*peerSetItem)))
		return true
	})
	return vs
}

func jsonDump(i interface{}) {
	_, file, line, _ := runtime.Caller(1)
	b, err := json.MarshalIndent(i, "", "  ")
	if err != nil {
		panic(err)
	}
	fmt.Printf(
		"%s %s:%d: %s\n",
		time.Now().Format("2006/01/02 15:04:05.000000"),
		path.Base(file),
		line, string(b),
	)
}

func streamToWriter(w io.Writer) {
	// b := make([]byte, 1024*384) // 30mbps
	// b := make([]byte, 1024*256) // 20mbps
	b := make([]byte, 1024*75) // 6mbps
	for i := 0; i < len(b); i += 1024 * 64 {
		n := 1024 * 64
		if i+n > len(b) {
			n = len(b) - i
		}
		if _, err := rand.Read(b[i : i+n]); err != nil {
			panic(err)
		}
	}
	for range time.NewTicker(time.Millisecond * 100).C {
		if _, err := w.Write(b); err != nil {
			return
		}
	}
}
