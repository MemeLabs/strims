package network

import (
	"bytes"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/petar/GoLLRB/llrb"
)

type certificateMap struct {
	mu sync.Mutex
	m  llrb.LLRB
}

func (c *certificateMap) Insert(certificate *pb.Certificate, networkID uint64) {
	c.mu.Lock()
	defer c.mu.Unlock()

	c.m.ReplaceOrInsert(&certificateMapItem{
		networkKey:  networkKeyForCertificate(certificate),
		networkID:   networkID,
		certificate: certificate,
		trusted:     isNetworkCertificateTrusted(certificate),
	})
}

func (c *certificateMap) Get(networkKey []byte) (*certificateMapItem, bool) {
	c.mu.Lock()
	defer c.mu.Unlock()

	item := c.m.Get(&certificateMapItem{networkKey: networkKey})
	if item == nil {
		return nil, false
	}
	return item.(*certificateMapItem), true
}

func (c *certificateMap) Delete(networkKey []byte) {
	c.mu.Lock()
	defer c.mu.Unlock()

	c.m.Delete(&certificateMapItem{networkKey: networkKey})
}

func (c *certificateMap) Keys() [][]byte {
	c.mu.Lock()
	defer c.mu.Unlock()

	keys := make([][]byte, 0, c.m.Len())
	c.m.AscendGreaterOrEqual(llrb.Inf(-1), func(i llrb.Item) bool {
		keys = append(keys, i.(*certificateMapItem).networkKey)
		return true
	})
	return keys
}

type certificateMapItem struct {
	networkKey  []byte
	networkID   uint64
	certificate *pb.Certificate
	trusted     bool
}

func (c *certificateMapItem) Less(o llrb.Item) bool {
	if o, ok := o.(*certificateMapItem); ok {
		return bytes.Compare(c.networkKey, o.networkKey) == -1
	}
	return !o.Less(c)
}
