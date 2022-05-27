// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package network

import (
	"bytes"
	"sync"

	"github.com/MemeLabs/strims/internal/dao"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/petar/GoLLRB/llrb"
)

func newCertificateMap() *certificateMap {
	return &certificateMap{
		loaded: make(chan struct{}),
	}
}

type certificateMap struct {
	mu         sync.Mutex
	m          llrb.LLRB
	loadedOnce sync.Once
	loaded     chan struct{}
}

func (c *certificateMap) SetLoaded() {
	c.loadedOnce.Do(func() { close(c.loaded) })
}

func (c *certificateMap) Loaded() <-chan struct{} {
	return c.loaded
}

func (c *certificateMap) Insert(network *networkv1.Network) {
	c.mu.Lock()
	defer c.mu.Unlock()

	c.m.ReplaceOrInsert(&certificateMapItem{
		networkKey:  dao.NetworkKey(network),
		networkID:   network.Id,
		certificate: network.Certificate,
		trusted:     isCertificateTrusted(network.Certificate),
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
	certificate *certificate.Certificate
	trusted     bool
}

func (c *certificateMapItem) Less(o llrb.Item) bool {
	if o, ok := o.(*certificateMapItem); ok {
		return bytes.Compare(c.networkKey, o.networkKey) == -1
	}
	return !o.Less(c)
}

// isPeerCertificateOwner checks that the key in the identity certificate
// received during the initial peer handshake matches the provided cert.
func isPeerCertificateOwner(peer *vnic.Peer, cert *certificate.Certificate) bool {
	return bytes.Equal(peer.Certificate.Key, cert.Key)
}

// isCertificateTrusted checks that the certificate is signed by the network's
// certificate authority. this filters provisional peer certificates used for
// invitations ex:
//
// pass: network member > network ca
// fail: provisional peer > network member > network ca
// fail: provisional peer > invitation > network member > network ca
func isCertificateTrusted(cert *certificate.Certificate) bool {
	return bytes.Equal(dao.CertificateNetworkKey(cert), cert.GetParent().GetKey()) && !isCertificateExpired(cert)
}

func isCertificateExpired(cert *certificate.Certificate) bool {
	return timeutil.Now().After(timeutil.Unix(int64(cert.GetNotAfter()), 0))
}

func nextCertificateRenewTime(network *networkv1.Network) timeutil.Time {
	if isCertificateSubjectMismatched(network) {
		// schedule immediate renewal we don't have a peer certificate for the
		// requested alias
		return timeutil.Now()
	}
	return timeutil.Unix(int64(network.Certificate.NotAfter), 0).Add(-certRenewScheduleAheadDuration)
}

func isCertificateSubjectMismatched(network *networkv1.Network) bool {
	return network.Alias != "" && network.Alias != network.Certificate.Subject
}
