// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package dao

import (
	"bytes"
	"encoding/base32"
	"encoding/binary"
	"net/url"
	"strings"

	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/strims/pkg/hashmap"
	"github.com/MemeLabs/strims/pkg/kv"
)

const (
	_ = iota + directoryNS
	directoryListingRecordNS
	directoryListingRecordNetworkNS
	directoryListingRecordListingNS
	directoryUserRecordNS
	directoryUserRecordPeerKeyNS
	directoryUserRecordNetworkNS
)

var DirectoryListingRecords = NewTable[networkv1directory.ListingRecord](directoryListingRecordNS, nil)

var GetDirectoryListingRecordsByNetworkID, GetDirectoryListingRecordsByNetwork, GetNetworkByDirectoryListingRecord = ManyToOne(
	directoryListingRecordNetworkNS,
	DirectoryListingRecords,
	Networks,
	(*networkv1directory.ListingRecord).GetNetworkId,
	&ManyToOneOptions{CascadeDelete: true},
)

const (
	_ byte = iota
	directoryListingChat
	directoryListingEmbed
	directoryListingMedia
	directoryListingService
)

func magnetURIID(s string) []byte {
	u, err := url.Parse(s)
	if err != nil {
		return nil
	}
	xt := u.Query().Get("xt")
	i := strings.LastIndex(xt, ":")
	b, _ := base32.StdEncoding.DecodeString(xt[i+1:])
	return b
}

func FormatDirectoryListingRecordListingKey(networkID uint64, m *networkv1directory.Listing) []byte {
	b := &bytes.Buffer{}
	binary.Write(b, binary.LittleEndian, networkID)
	switch c := m.Content.(type) {
	case *networkv1directory.Listing_Chat_:
		b.WriteByte(directoryListingChat)
		b.Write(c.Chat.Key)
	case *networkv1directory.Listing_Embed_:
		b.WriteByte(directoryListingEmbed)
		binary.Write(b, binary.LittleEndian, c.Embed.Service)
		b.WriteString(c.Embed.Id)
	case *networkv1directory.Listing_Media_:
		b.WriteByte(directoryListingMedia)
		b.Write(magnetURIID(c.Media.SwarmUri))
	case *networkv1directory.Listing_Service_:
		b.WriteByte(directoryListingService)
		b.WriteString(c.Service.Type)
		b.WriteString(c.Service.SwarmUri)
	}
	return b.Bytes()
}

func DirectoryListingsEqual(a, b *networkv1directory.Listing) bool {
	switch ac := a.Content.(type) {
	case *networkv1directory.Listing_Chat_:
		bc, ok := a.Content.(*networkv1directory.Listing_Chat_)
		return ok && bytes.Equal(ac.Chat.Key, bc.Chat.Key)
	case *networkv1directory.Listing_Embed_:
		bc, ok := a.Content.(*networkv1directory.Listing_Embed_)
		return ok && ac.Embed.Service == bc.Embed.Service && ac.Embed.Id == bc.Embed.Id
	case *networkv1directory.Listing_Media_:
		bc, ok := a.Content.(*networkv1directory.Listing_Media_)
		return ok && bytes.Equal(magnetURIID(ac.Media.SwarmUri), magnetURIID(bc.Media.SwarmUri))
	case *networkv1directory.Listing_Service_:
		bc, ok := a.Content.(*networkv1directory.Listing_Service_)
		return ok && ac.Service.Type == bc.Service.Type
	}
	return false
}

func directoryListingKey(m *networkv1directory.ListingRecord) []byte {
	return FormatDirectoryListingRecordListingKey(m.NetworkId, m.Listing)
}

var DirectoryListingRecordsByListing = NewUniqueIndex(
	directoryListingRecordListingNS,
	DirectoryListingRecords,
	directoryListingKey,
	byteIdentity,
	nil,
)

func NewDirectoryListingRecordCache(s kv.RWStore, opt *CacheStoreOptions) (c DirectoryListingRecordCache) {
	c.CacheStore, c.ByID = newCacheStore[networkv1directory.ListingRecord](s, DirectoryListingRecords, opt)
	c.ByListing = NewCacheIndex(
		c.CacheStore,
		DirectoryListingRecordsByListing.Get,
		directoryListingKey,
		hashmap.NewByteInterface[[]byte],
	)
	return
}

type DirectoryListingRecordCache struct {
	*CacheStore[networkv1directory.ListingRecord, *networkv1directory.ListingRecord]
	ByID      CacheAccessor[uint64, networkv1directory.ListingRecord, *networkv1directory.ListingRecord]
	ByListing CacheAccessor[[]byte, networkv1directory.ListingRecord, *networkv1directory.ListingRecord]
}

var DirectoryUserRecords = NewTable[networkv1directory.UserRecord](directoryUserRecordNS, nil)

var GetDirectoryUserRecordsByNetworkID, GetDirectoryUserRecordsByNetwork, GetNetworkByDirectoryUserRecord = ManyToOne(
	directoryUserRecordNetworkNS,
	DirectoryUserRecords,
	Networks,
	(*networkv1directory.UserRecord).GetNetworkId,
	&ManyToOneOptions{CascadeDelete: true},
)

func FormatDirectoryUserRecordPeerKeyKey(networkID uint64, peerKey []byte) []byte {
	b := make([]byte, 8+len(peerKey))
	binary.BigEndian.PutUint64(b, networkID)
	copy(b[8:], peerKey)
	return b
}

func directoryUserRecordPeerKeyKey(m *networkv1directory.UserRecord) []byte {
	return FormatDirectoryUserRecordPeerKeyKey(m.NetworkId, m.PeerKey)
}

var DirectoryUserRecordsByPeerKey = NewUniqueIndex(
	directoryUserRecordPeerKeyNS,
	DirectoryUserRecords,
	directoryUserRecordPeerKeyKey,
	byteIdentity,
	nil,
)

func NewDirectoryUserRecordCache(s kv.RWStore, opt *CacheStoreOptions) (c DirectoryUserRecordCache) {
	c.CacheStore, c.ByID = newCacheStore[networkv1directory.UserRecord](s, DirectoryUserRecords, opt)
	c.ByPeerKey = NewCacheIndex(
		c.CacheStore,
		DirectoryUserRecordsByPeerKey.Get,
		directoryUserRecordPeerKeyKey,
		hashmap.NewByteInterface[[]byte],
	)
	return
}

type DirectoryUserRecordCache struct {
	*CacheStore[networkv1directory.UserRecord, *networkv1directory.UserRecord]
	ByID      CacheAccessor[uint64, networkv1directory.UserRecord, *networkv1directory.UserRecord]
	ByPeerKey CacheAccessor[[]byte, networkv1directory.UserRecord, *networkv1directory.UserRecord]
}

func NewDirectoryListingRecord(s IDGenerator, networkID uint64, listing *networkv1directory.Listing) (*networkv1directory.ListingRecord, error) {
	id, err := s.GenerateID()
	if err != nil {
		return nil, err
	}
	return &networkv1directory.ListingRecord{
		Id:        id,
		NetworkId: networkID,
		Listing:   listing,
	}, nil
}

func NewDirectoryUserRecord(s IDGenerator, networkID uint64, peerKey []byte) (*networkv1directory.UserRecord, error) {
	id, err := s.GenerateID()
	if err != nil {
		return nil, err
	}
	return &networkv1directory.UserRecord{
		Id:        id,
		NetworkId: networkID,
		PeerKey:   peerKey,
	}, nil
}
