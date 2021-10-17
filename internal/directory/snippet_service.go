package directory

import (
	"bytes"
	"context"
	"errors"
	"sort"
	"sync"

	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/event"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/sortutil"
	"github.com/petar/GoLLRB/llrb"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/types/known/wrapperspb"
)

type snippetServer struct {
	ctx      context.Context
	dialer   network.Dialer
	snippets *snippetMap
	service  snippetService
}

func (s *snippetServer) UpdateSnippet(swarmID ppspp.SwarmID, snippet *networkv1directory.ListingSnippet) {
	s.snippets.Update(swarmID, snippet)
}

func (s *snippetServer) DeleteSnippet(swarmID ppspp.SwarmID) {
	s.snippets.Delete(swarmID)
}

func (s *snippetServer) start() {
	// s.dialer.ServerDialer(networkKey []byte, port uint16, publisher dialer.HostAddrPublisher)
}

func (s *snippetServer) stop() {

}

var _ networkv1directory.DirectorySnippetService = &snippetService{}

type snippetService struct {
	transfer   transfer.Control
	snippets   *snippetMap
	networkKey []byte
}

func (s *snippetService) Subscribe(ctx context.Context, req *networkv1directory.SnippetSubscribeRequest) (<-chan *networkv1directory.SnippetSubscribeResponse, error) {
	if !s.transfer.IsPublished(transfer.NewID(req.SwarmId, nil), s.networkKey) {
		return nil, errors.New("snippet not found")
	}

	ch := make(chan *networkv1directory.SnippetSubscribeResponse, 16)

	snippet, ok := s.snippets.Get(req.SwarmId)
	if !ok {
		return nil, errors.New("snippet not found")
	}

	go func() {
		defer close(ch)

		deltas := make(chan *networkv1directory.ListingSnippetDelta, 16)
		snippet.Notify(deltas)
		defer snippet.StopNotifying(deltas)

		for {
			select {
			case delta, ok := <-deltas:
				if !ok {
					return
				}
				ch <- &networkv1directory.SnippetSubscribeResponse{
					SnippetDelta: delta,
				}
			case <-ctx.Done():
				return
			}
		}
	}()

	return ch, nil
}

type snippetMap struct {
	snippetsLock sync.Mutex
	snippets     llrb.LLRB
}

func (m *snippetMap) Update(swarmID ppspp.SwarmID, snippet *networkv1directory.ListingSnippet) {
	m.snippetsLock.Lock()
	defer m.snippetsLock.Unlock()

	it, ok := m.snippets.Get(&snippetItem{id: swarmID.Binary()}).(*snippetItem)
	if !ok {
		it = newSnippetItem(swarmID.Binary())
		m.snippets.ReplaceOrInsert(it)
	}

	it.Update(snippet)
}

func (m *snippetMap) Delete(swarmID ppspp.SwarmID) {
	m.snippetsLock.Lock()
	defer m.snippetsLock.Unlock()

	m.snippets.Delete(&snippetItem{id: swarmID.Binary()})
}

func (m *snippetMap) Get(swarmID ppspp.SwarmID) (*snippetItem, bool) {
	it, ok := m.snippets.Get(&snippetItem{id: swarmID}).(*snippetItem)
	return it, ok
}

func newSnippetItem(swarmID ppspp.SwarmID) *snippetItem {
	return &snippetItem{
		id:      swarmID.Binary(),
		snippet: &networkv1directory.ListingSnippet{},
	}
}

type snippetItem struct {
	id      []byte
	lock    sync.Mutex
	snippet *networkv1directory.ListingSnippet
	deltas  event.Observer
}

func (i *snippetItem) Less(o llrb.Item) bool {
	if o, ok := o.(*snippetItem); ok {
		return bytes.Compare(i.id, o.id) == -1
	}
	return !o.Less(i)
}

func (i *snippetItem) Notify(ch chan *networkv1directory.ListingSnippetDelta) {
	i.lock.Lock()
	defer i.lock.Unlock()

	delta := &networkv1directory.ListingSnippetDelta{
		Title:       &wrapperspb.StringValue{Value: i.snippet.Title},
		Description: &wrapperspb.StringValue{Value: i.snippet.Description},
		TagsOneof:   &networkv1directory.ListingSnippetDelta_Tags_{Tags: &networkv1directory.ListingSnippetDelta_Tags{Tags: i.snippet.Tags}},
		Category:    &wrapperspb.StringValue{Value: i.snippet.Category},
		ChannelName: &wrapperspb.StringValue{Value: i.snippet.ChannelName},
		ViewerCount: &wrapperspb.UInt64Value{Value: i.snippet.ViewerCount},
		Live:        &wrapperspb.BoolValue{Value: i.snippet.Live},
		IsMature:    &wrapperspb.BoolValue{Value: i.snippet.IsMature},
		Key:         &wrapperspb.BytesValue{Value: i.snippet.Key},
		Signature:   &wrapperspb.BytesValue{Value: i.snippet.Signature},
	}
	changed := true

	if i.snippet.ChannelLogo != nil {
		delta.ChannelLogoOneof = &networkv1directory.ListingSnippetDelta_ChannelLogo{ChannelLogo: i.snippet.ChannelLogo}

		// TODO: vpn message fragmentation.
		// avoid exceeding MTU by returning channel logo and thumbnail separately.
		ch <- delta
		delta = &networkv1directory.ListingSnippetDelta{}
		changed = false
	}

	if i.snippet.Thumbnail != nil {
		delta.ThumbnailOneof = &networkv1directory.ListingSnippetDelta_Thumbnail{Thumbnail: i.snippet.Thumbnail}
		changed = true
	}

	if changed {
		ch <- delta
	}

	i.deltas.Notify(ch)
}

func (i *snippetItem) StopNotifying(ch chan *networkv1directory.ListingSnippetDelta) {
	i.deltas.StopNotifying(ch)
}

func (i *snippetItem) Update(snippet *networkv1directory.ListingSnippet) {
	i.lock.Lock()
	defer i.lock.Unlock()

	delta := &networkv1directory.ListingSnippetDelta{}
	var changed bool

	if i.snippet.Title != snippet.Title {
		delta.Title = &wrapperspb.StringValue{Value: snippet.Title}
		i.snippet.Title = snippet.Title
		changed = true
	}

	if i.snippet.Description != snippet.Description {
		delta.Description = &wrapperspb.StringValue{Value: snippet.Description}
		i.snippet.Description = snippet.Description
		changed = true
	}

	tags := make([]string, len(snippet.Tags))
	copy(tags, snippet.Tags)
	sort.Strings(tags)
	if !sortutil.EqualStrings(tags, snippet.Tags) {
		delta.TagsOneof = &networkv1directory.ListingSnippetDelta_Tags_{Tags: &networkv1directory.ListingSnippetDelta_Tags{Tags: snippet.Tags}}
		i.snippet.Tags = tags
		changed = true
	}

	if i.snippet.Category != snippet.Category {
		delta.Category = &wrapperspb.StringValue{Value: snippet.Category}
		i.snippet.Category = snippet.Category
		changed = true
	}

	if i.snippet.ChannelName != snippet.ChannelName {
		delta.ChannelName = &wrapperspb.StringValue{Value: snippet.ChannelName}
		i.snippet.ChannelName = snippet.ChannelName
		changed = true
	}

	if i.snippet.ViewerCount != snippet.ViewerCount {
		delta.ViewerCount = &wrapperspb.UInt64Value{Value: snippet.ViewerCount}
		i.snippet.ViewerCount = snippet.ViewerCount
		changed = true
	}

	if i.snippet.Live != snippet.Live {
		delta.Live = &wrapperspb.BoolValue{Value: snippet.Live}
		i.snippet.Live = snippet.Live
		changed = true
	}

	if i.snippet.IsMature != snippet.IsMature {
		delta.IsMature = &wrapperspb.BoolValue{Value: snippet.IsMature}
		i.snippet.IsMature = snippet.IsMature
		changed = true
	}

	if !bytes.Equal(i.snippet.Key, snippet.Key) {
		delta.Key = &wrapperspb.BytesValue{Value: snippet.Key}
		i.snippet.Key = make([]byte, len(snippet.Key))
		copy(i.snippet.Key, snippet.Key)
		changed = true
	}

	if !bytes.Equal(i.snippet.Signature, snippet.Signature) {
		delta.Signature = &wrapperspb.BytesValue{Value: snippet.Signature}
		i.snippet.Signature = make([]byte, len(snippet.Signature))
		copy(i.snippet.Signature, snippet.Signature)
		changed = true
	}

	if !proto.Equal(i.snippet.ChannelLogo, snippet.ChannelLogo) {
		delta.ChannelLogoOneof = &networkv1directory.ListingSnippetDelta_ChannelLogo{ChannelLogo: snippet.ChannelLogo}
		i.snippet.ChannelLogo = proto.Clone(snippet.ChannelLogo).(*networkv1directory.ListingSnippetImage)

		i.deltas.Emit(delta)
		delta = &networkv1directory.ListingSnippetDelta{}
		changed = false
	}

	if !proto.Equal(i.snippet.Thumbnail, snippet.Thumbnail) {
		delta.ThumbnailOneof = &networkv1directory.ListingSnippetDelta_Thumbnail{Thumbnail: snippet.Thumbnail}
		i.snippet.Thumbnail = proto.Clone(snippet.Thumbnail).(*networkv1directory.ListingSnippetImage)
		changed = true
	}

	if changed {
		i.deltas.Emit(delta)
	}
}
