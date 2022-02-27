package vpn

import (
	"bytes"
	"context"
	"crypto/sha256"
	"errors"
	"log"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	vpnv1 "github.com/MemeLabs/go-ppspp/pkg/apis/vpn/v1"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/randutil"
	"github.com/MemeLabs/go-ppspp/pkg/syncutil"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

const peerIndexSearchResponseSize = 5
const peerIndexPublishInterval = 10 * time.Minute
const peerIndexGCInterval = time.Minute
const peerIndexMaxRecordAge = 30 * time.Minute
const peerIndexMaxSize = 5120
const peerIndexSearchTimeout = time.Minute

// PeerIndex ...
type PeerIndex interface {
	Publish(ctx context.Context, key, salt []byte, port uint16) error
	Search(ctx context.Context, key, salt []byte) (<-chan *PeerIndexHost, error)
}

func newPeerIndex(logger *zap.Logger, n *Network) *peerIndex {
	store := newPeerIndexStore(logger)
	stopGC := timeutil.DefaultTickEmitter.Subscribe(peerIndexGCInterval, store.Prune, nil)

	return &peerIndex{
		logger:  logger,
		store:   store,
		stopGC:  stopGC,
		network: n,
	}
}

// PeerIndex ...
type peerIndex struct {
	logger              *zap.Logger
	network             *Network
	store               *peerIndexStore
	stopGC              timeutil.StopFunc
	searchResponseChans syncutil.Map[uint64, chan *PeerIndexHost]
}

func (s *peerIndex) Close() {
	s.stopGC()
}

// HandleMessage ...
func (s *peerIndex) HandleMessage(msg *Message) error {
	var m vpnv1.PeerIndexMessage
	if err := proto.Unmarshal(msg.Body, &m); err != nil {
		return err
	}

	switch b := m.Body.(type) {
	case *vpnv1.PeerIndexMessage_Publish_:
		return s.handlePublish(b.Publish.Record, msg.SrcHostID())
	case *vpnv1.PeerIndexMessage_Unpublish_:
		return s.handleUnpublish(b.Unpublish.Record, msg.SrcHostID())
	case *vpnv1.PeerIndexMessage_SearchRequest_:
		return s.handleSearchRequest(b.SearchRequest, msg.SrcHostID())
	case *vpnv1.PeerIndexMessage_SearchResponse_:
		return s.handleSearchResponse(b.SearchResponse)
	default:
		return errors.New("unexpected message type")
	}
}

func (s *peerIndex) handlePublish(r *vpnv1.PeerIndexMessage_Record, hostID kademlia.ID) error {
	if err := dao.VerifyMessage(r); err != nil {
		return err
	}
	if !bytes.Equal(r.HostId, hostID.Bytes(nil)) {
		return errors.New("host id mismatch")
	}

	s.store.Upsert(r, hostID)
	return nil
}

func (s *peerIndex) handleUnpublish(r *vpnv1.PeerIndexMessage_Record, hostID kademlia.ID) error {
	if err := dao.VerifyMessage(r); err != nil {
		return err
	}
	if !bytes.Equal(r.HostId, hostID.Bytes(nil)) {
		return errors.New("host id mismatch")
	}

	s.store.Remove(r)
	return nil
}

func (s *peerIndex) handleSearchRequest(m *vpnv1.PeerIndexMessage_SearchRequest, hostID kademlia.ID) error {
	records := s.store.Get(m.Hash, hostID, s.network.HasPeer)
	if len(records) == 0 {
		return nil
	}

	msg := &vpnv1.PeerIndexMessage{
		Body: &vpnv1.PeerIndexMessage_SearchResponse_{
			SearchResponse: &vpnv1.PeerIndexMessage_SearchResponse{
				RequestId: m.RequestId,
				Records:   records,
			},
		},
	}
	return s.network.SendProto(hostID, vnic.PeerIndexPort, vnic.PeerIndexPort, msg)
}

func (s *peerIndex) handleSearchResponse(m *vpnv1.PeerIndexMessage_SearchResponse) error {
	ch, ok := s.searchResponseChans.Get(m.RequestId)
	if !ok {
		return nil
	}

	for _, r := range m.Records {
		if dao.VerifyMessage(r) != nil {
			continue
		}

		// TODO: check hash
		// TODO: check timestamp

		h, err := newPeerIndexHostFromRecord(r)
		if err != nil {
			continue
		}

		select {
		case ch <- h:
		default:
		}
	}
	return nil
}

func (s *peerIndex) Publish(ctx context.Context, key, salt []byte, port uint16) error {
	s.logger.Debug(
		"publishing peer index item",
		logutil.ByteHex("key", key),
		logutil.ByteHex("salt", salt),
		zap.Uint16("port", port),
	)
	_, err := newPeerIndexPublisher(ctx, s.logger, s.network, key, salt, port)
	return err
}

func (s *peerIndex) Search(ctx context.Context, key, salt []byte) (<-chan *PeerIndexHost, error) {
	hash := peerIndexRecordHash(key, salt)
	target, err := kademlia.UnmarshalID(hash)
	if err != nil {
		return nil, err
	}

	rid, err := randutil.Uint64()
	if err != nil {
		return nil, err
	}

	hosts := make(chan *PeerIndexHost, 32)
	s.searchResponseChans.Set(rid, hosts)
	cleanup := func() {
		s.searchResponseChans.Delete(rid)
		close(hosts)
	}

	msg := &vpnv1.PeerIndexMessage{
		Body: &vpnv1.PeerIndexMessage_SearchRequest_{
			SearchRequest: &vpnv1.PeerIndexMessage_SearchRequest{
				RequestId: rid,
				Hash:      hash,
			},
		},
	}
	if err := s.network.SendProtoWithFlags(target, vnic.PeerIndexPort, vnic.PeerIndexPort, msg, Mbroadcast); err != nil {
		cleanup()
		return nil, err
	}

	ctx, cancel := context.WithTimeout(ctx, peerIndexSearchTimeout)
	go func() {
		<-ctx.Done()
		cancel()
		cleanup()
	}()

	return hosts, nil
}

type peerIndexItem struct {
	record *vpnv1.PeerIndexMessage_Record
	hostID kademlia.ID
	time   timeutil.Time
	prev   *peerIndexItem
	next   *peerIndexItem
}

func (i *peerIndexItem) Less(o llrb.Item) bool {
	if o, ok := o.(*peerIndexItem); ok {
		if d := bytes.Compare(i.record.Hash, o.record.Hash); d != 0 {
			return d == -1
		}
		return bytes.Compare(i.record.HostId, o.record.HostId) == -1
	}
	return !o.Less(i)
}

func (i *peerIndexItem) ID() kademlia.ID {
	return i.hostID
}

func newPeerIndexStore(logger *zap.Logger) *peerIndexStore {
	return &peerIndexStore{
		logger: logger,
		items:  llrb.New(),
	}
}

type peerIndexStore struct {
	logger *zap.Logger
	lock   sync.Mutex
	items  *llrb.LLRB
	tail   *peerIndexItem
	head   *peerIndexItem
}

func (s *peerIndexStore) Upsert(r *vpnv1.PeerIndexMessage_Record, hostID kademlia.ID) {
	s.lock.Lock()
	defer s.lock.Unlock()

	i := &peerIndexItem{record: r}
	if ii := s.items.Get(i); ii != nil {
		i = ii.(*peerIndexItem)
		s.remove(i)

		s.logger.Debug(
			"updating peer index item",
			logutil.ByteHex("hash", r.Hash),
			logutil.ByteHex("host", r.HostId),
			zap.Uint32("port", r.Port),
		)
	} else {
		i.hostID = hostID
		s.items.ReplaceOrInsert(i)

		s.logger.Debug(
			"inserting peer index item",
			logutil.ByteHex("hash", r.Hash),
			logutil.ByteHex("host", r.HostId),
			zap.Uint32("port", r.Port),
		)
	}
	s.push(i)
}

func (s *peerIndexStore) Remove(r *vpnv1.PeerIndexMessage_Record) {
	s.lock.Lock()
	defer s.lock.Unlock()

	if ii := s.items.Delete(&peerIndexItem{record: r}); ii != nil {
		s.remove(ii.(*peerIndexItem))

		s.logger.Debug(
			"removing peer index item",
			logutil.ByteHex("hash", r.Hash),
			logutil.ByteHex("host", r.HostId),
			zap.Uint32("port", r.Port),
		)
	}
}

func (s *peerIndexStore) Get(hash []byte, hostID kademlia.ID, verifyHostID func(kademlia.ID) bool) []*vpnv1.PeerIndexMessage_Record {
	s.lock.Lock()
	defer s.lock.Unlock()

	min := &peerIndexItem{record: &vpnv1.PeerIndexMessage_Record{Hash: hash}}
	max := &peerIndexItem{record: &vpnv1.PeerIndexMessage_Record{Hash: hash, HostId: kademlia.MaxID.Bytes(nil)}}

	f := kademlia.NewFilter(hostID)
	defer f.Free()
	s.items.AscendRange(min, max, func(i llrb.Item) bool {
		f.Push(i.(*peerIndexItem))
		return true
	})

	records := make([]*vpnv1.PeerIndexMessage_Record, 0, peerIndexSearchResponseSize)
	f.Each(func(ii kademlia.Interface) bool {
		i := ii.(*peerIndexItem)
		if !hostID.Equals(i.hostID) && verifyHostID(i.hostID) {
			records = append(records, i.record)
		}
		return len(records) < peerIndexSearchResponseSize
	})
	return records
}

func (s *peerIndexStore) Prune(t timeutil.Time) {
	s.lock.Lock()
	defer s.lock.Unlock()

	eol := t.Add(-peerIndexMaxRecordAge)
	for s.head != nil && s.head.time < eol {
		s.items.Delete(s.head)
		s.remove(s.head)
	}
}

func (s *peerIndexStore) remove(i *peerIndexItem) {
	if s.tail == i {
		s.tail = i.prev
	}
	if s.head == i {
		s.head = i.next
	}
	if i.prev != nil {
		i.prev.next = i.next
	}
	if i.next != nil {
		i.next.prev = i.prev
	}
}

func (s *peerIndexStore) push(i *peerIndexItem) {
	i.time = timeutil.Now()
	i.next = s.head
	i.prev = nil

	if i.next != nil {
		i.next.prev = i
	}

	s.head = i
	if s.tail == nil {
		s.tail = i
	}
}

func peerIndexRecordHash(key, salt []byte) []byte {
	hash := sha256.New()
	if _, err := hash.Write(key); err != nil {
		log.Println(err)
	}
	if _, err := hash.Write(salt); err != nil {
		log.Println(err)
	}
	return hash.Sum(nil)
}

func newPeerIndexHostFromRecord(r *vpnv1.PeerIndexMessage_Record) (*PeerIndexHost, error) {
	hostID, err := kademlia.UnmarshalID(r.HostId)
	if err != nil {
		return nil, err
	}
	return &PeerIndexHost{
		Timestamp: timeutil.Unix(r.Timestamp, 0),
		HostID:    hostID,
		Port:      uint16(r.Port),
	}, nil
}

// PeerIndexHost ...
type PeerIndexHost struct {
	Timestamp timeutil.Time
	HostID    kademlia.ID
	Port      uint16
}

func newPeerIndexPublisher(ctx context.Context, logger *zap.Logger, network *Network, key, salt []byte, port uint16) (*peerIndexPublisher, error) {
	hash := peerIndexRecordHash(key, salt)

	record := &vpnv1.PeerIndexMessage_Record{
		Hash:   hash,
		HostId: network.host.ID().Bytes(nil),
		Port:   uint32(port),
	}

	p := &peerIndexPublisher{
		logger:  logger,
		record:  record,
		network: network,
	}

	go p.publish(timeutil.Now())
	timeutil.DefaultTickEmitter.SubscribeCtx(ctx, peerIndexPublishInterval, p.publish, p.unpublish)

	return p, nil
}

type peerIndexPublisher struct {
	logger  *zap.Logger
	record  *vpnv1.PeerIndexMessage_Record
	network *Network
}

func (p *peerIndexPublisher) update() error {
	p.record.Timestamp = timeutil.Now().Unix()
	if err := dao.SignMessage(p.record, p.network.host.Key()); err != nil {
		return err
	}
	return nil
}

func (p *peerIndexPublisher) publish(t timeutil.Time) {
	if err := p.update(); err != nil {
		return
	}

	msg := &vpnv1.PeerIndexMessage{
		Body: &vpnv1.PeerIndexMessage_Publish_{
			Publish: &vpnv1.PeerIndexMessage_Publish{
				Record: p.record,
			},
		},
	}
	if err := p.network.BroadcastProtoWithFlags(vnic.PeerIndexPort, vnic.PeerIndexPort, msg, Mnorelay); err != nil {
		p.logger.Debug(
			"error publishing peer index item",
			zap.Error(err),
		)
	}
}

func (p *peerIndexPublisher) unpublish() {
	if err := p.update(); err != nil {
		return
	}

	msg := &vpnv1.PeerIndexMessage{
		Body: &vpnv1.PeerIndexMessage_Unpublish_{
			Unpublish: &vpnv1.PeerIndexMessage_Unpublish{
				Record: p.record,
			},
		},
	}
	if err := p.network.BroadcastProtoWithFlags(vnic.PeerIndexPort, vnic.PeerIndexPort, msg, Mnorelay); err != nil {
		p.logger.Debug(
			"error unpublishing peer index item",
			zap.Error(err),
		)
	}
}
