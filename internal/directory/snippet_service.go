package directory

import (
	"bytes"
	"context"
	"errors"
	"sync"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/errutil"
	"github.com/MemeLabs/go-ppspp/pkg/event"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

type snippetServer struct {
	logger   *zap.Logger
	dialer   network.Dialer
	transfer transfer.Control
	snippets *snippetMap
	servers  map[uint64]context.CancelFunc
}

func (s *snippetServer) UpdateSnippet(swarmID ppspp.SwarmID, snippet *networkv1directory.ListingSnippet) {
	s.snippets.Update(swarmID, snippet)
}

func (s *snippetServer) DeleteSnippet(swarmID ppspp.SwarmID) {
	s.snippets.Delete(swarmID)
}

func (s *snippetServer) start(ctx context.Context, network *networkv1.Network) {
	networkKey := dao.NetworkKey(network)

	server := errutil.Must(s.dialer.ServerWithHostAddr(ctx, networkKey, vnic.SnippetPort))

	networkv1directory.RegisterDirectorySnippetService(server, &snippetService{
		logger:     s.logger.With(logutil.ByteHex("network", networkKey)),
		transfer:   s.transfer,
		snippets:   s.snippets,
		networkKey: networkKey,
	})

	ctx, cancel := context.WithCancel(ctx)

	go func() {
		s.logger.Debug(
			"starting directory snippet server",
			logutil.ByteHex("network", networkKey),
		)
		err := server.Listen(ctx)
		s.logger.Debug(
			"directory snippet server closed",
			logutil.ByteHex("network", networkKey),
			zap.Error(err),
		)
	}()

	s.servers[network.Id] = cancel

	// s.dialer.ServerDialer(networkKey []byte, port uint16, publisher dialer.HostAddrPublisher)
}

func (s *snippetServer) stop(id uint64) {
	if cancel, ok := s.servers[id]; ok {
		delete(s.servers, id)
		cancel()
	}
}

var _ networkv1directory.DirectorySnippetService = &snippetService{}

type snippetService struct {
	logger     *zap.Logger
	transfer   transfer.Control
	snippets   *snippetMap
	networkKey []byte
}

func (s *snippetService) Subscribe(ctx context.Context, req *networkv1directory.SnippetSubscribeRequest) (<-chan *networkv1directory.SnippetSubscribeResponse, error) {
	// TODO: deny requests not from directory service hosts...
	// token auth? signed message(swarmID|timestamp, profile.Key)
	// local acl maintained via publish?
	// can we id them based on their cert? if we save the remote profile key from the publish rpc?
	// look up directory in dht? we should have it cached...

	logger := s.logger.With(zap.Stringer("swarmID", ppspp.SwarmID(req.SwarmId)))

	if !s.transfer.IsPublished(transfer.NewID(req.SwarmId, nil), s.networkKey) {
		return nil, errors.New("snippet not found")
	}

	logger.Debug("received snippet service subscription")

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

	it, ok := m.snippets.Get(&snippetItem{id: swarmID}).(*snippetItem)
	if !ok {
		it = newSnippetItem(swarmID)
		m.snippets.ReplaceOrInsert(it)
	}

	it.Update(snippet)
}

func (m *snippetMap) Delete(swarmID ppspp.SwarmID) {
	m.snippetsLock.Lock()
	defer m.snippetsLock.Unlock()

	it, ok := m.snippets.Delete(&snippetItem{id: swarmID}).(*snippetItem)
	if ok {
		it.Destroy()
	}
}

func (m *snippetMap) Get(swarmID ppspp.SwarmID) (*snippetItem, bool) {
	it, ok := m.snippets.Get(&snippetItem{id: swarmID}).(*snippetItem)
	return it, ok
}

func newSnippetItem(swarmID ppspp.SwarmID) *snippetItem {
	return &snippetItem{
		id:      swarmID,
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

	delta := diffSnippets(nilSnippet, i.snippet)

	if delta.GetThumbnail() != nil && delta.GetChannelLogo() != nil {
		ch <- &networkv1directory.ListingSnippetDelta{ThumbnailOneof: delta.ThumbnailOneof}
		delta.ThumbnailOneof = nil
	}
	ch <- delta

	i.deltas.Notify(ch)
}

func (i *snippetItem) StopNotifying(ch chan *networkv1directory.ListingSnippetDelta) {
	i.deltas.StopNotifying(ch)
}

func (i *snippetItem) Update(snippet *networkv1directory.ListingSnippet) {
	i.lock.Lock()
	defer i.lock.Unlock()

	delta := diffSnippets(i.snippet, snippet)
	if isNilSnippetDelta(delta) {
		return
	}
	mergeSnippet(i.snippet, delta)

	if delta.GetThumbnail() != nil && delta.GetChannelLogo() != nil {
		i.deltas.Emit(&networkv1directory.ListingSnippetDelta{ThumbnailOneof: delta.ThumbnailOneof})
		delta.ThumbnailOneof = nil
	}
	i.deltas.Emit(delta)
}

func (i *snippetItem) Destroy() {
	i.deltas.Close()
}
