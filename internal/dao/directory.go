package dao

import (
	"bytes"
	"encoding/binary"

	"google.golang.org/protobuf/proto"

	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
)

const (
	_ = iota + directoryNS
	directoryListingRecordNS
	directoryListingRecordNetworkNS
	directoryListingRecordListingNS
)

var DirectoryListingRecords = NewTable(
	directoryListingRecordNS,
	&TableOptions[networkv1directory.ListingRecord]{
		ObserveChange: func(m, p *networkv1directory.ListingRecord) proto.Message {
			return &networkv1directory.ListingRecordChangeEvent{Record: m}
		},
	},
)

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

func FormatDirectoryListingRecordListingKey(m *networkv1directory.Listing) []byte {
	b := &bytes.Buffer{}
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
		uri, _ := ppspp.ParseURI(c.Media.SwarmUri)
		b.Write(uri.ID)
	case *networkv1directory.Listing_Service_:
		b.WriteByte(directoryListingService)
		b.WriteString(c.Service.Type)
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
		if ok {
			aURI, _ := ppspp.ParseURI(ac.Media.SwarmUri)
			bURI, _ := ppspp.ParseURI(bc.Media.SwarmUri)
			return aURI.ID.Equals(bURI.ID)
		}
	case *networkv1directory.Listing_Service_:
		bc, ok := a.Content.(*networkv1directory.Listing_Service_)
		return ok && ac.Service.Type == bc.Service.Type
	}
	return false
}

var GetDirectoryListingRecordByListing = UniqueIndex(
	directoryListingRecordListingNS,
	DirectoryListingRecords,
	func(m *networkv1directory.ListingRecord) []byte {
		return FormatDirectoryListingRecordListingKey(m.Listing)
	},
	nil,
)

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
