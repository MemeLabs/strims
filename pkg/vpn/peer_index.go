package vpn

import (
	"bytes"
	"context"
	"crypto/ed25519"
	"crypto/sha1"
	"encoding/binary"
	"errors"
	"sync"
	"sync/atomic"
	"time"
	"unsafe"

	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/pool"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

const peerIndexSearchResponseSize = 5
const peerIndexPublishInterval = 10 * time.Minute
const peerIndexDiscardInterval = time.Minute
const peerIndexMaxRecordAge = 30 * time.Minute
const peerIndexMaxSize = 5120

var nextPeerIndexID uint32

// PeerIndex ...
type PeerIndex interface {
	Publish(ctx context.Context, key, salt []byte, port uint16) error
	Search(ctx context.Context, key, salt []byte) (<-chan *PeerIndexHost, error)
	HandleMessage(msg *Message) (forward bool, err error)
}

// NewPeerIndex ...
func NewPeerIndex(logger *zap.Logger, n *Network, store *PeerIndexStore) PeerIndex {
	id := atomic.AddUint32(&nextPeerIndexID, 1)
	if id == 0 {
		panic("peer index id overflow")
	}

	return &peerIndex{
		logger:  logger,
		id:      id,
		store:   store,
		network: n,
	}
}

type peerIndex struct {
	logger              *zap.Logger
	id                  uint32
	store               *PeerIndexStore
	network             *Network
	searchResponseChans sync.Map
}

func (s *peerIndex) HandleMessage(msg *Message) (forward bool, err error) {
	var m pb.PeerIndexMessage
	if err := proto.Unmarshal(msg.Body, &m); err != nil {
		return true, err
	}

	switch b := m.Body.(type) {
	case *pb.PeerIndexMessage_Publish_:
		err = s.handlePublish(b.Publish.Record)
	case *pb.PeerIndexMessage_Unpublish_:
		err = s.handleUnpublish(b.Unpublish.Record)
	case *pb.PeerIndexMessage_SearchRequest_:
		originHostID := s.network.host.ID()
		if len(msg.Trailers) != 0 {
			originHostID = msg.Trailers[0].HostID
		}
		err = s.handleSearchRequest(b.SearchRequest, originHostID)
	case *pb.PeerIndexMessage_SearchResponse_:
		err = s.handleSearchResponse(b.SearchResponse)
	default:
		err = errors.New("unexpected message type")
	}

	return true, err
}

func (s *peerIndex) handlePublish(r *pb.PeerIndexMessage_Record) error {
	if !verifyPeerIndexRecord(r) {
		return errors.New("invalid record signature")
	}

	return s.store.Insert(s.id, r)
}

func (s *peerIndex) handleUnpublish(r *pb.PeerIndexMessage_Record) error {
	if !verifyPeerIndexRecord(r) {
		return errors.New("invalid record signature")
	}

	return s.store.Remove(s.id, r)
}

func (s *peerIndex) handleSearchRequest(m *pb.PeerIndexMessage_SearchRequest, originHostID kademlia.ID) error {
	records := s.store.Closest(s.id, originHostID, m.Hash)
	if len(records) == 0 {
		return nil
	}

	msg := &pb.PeerIndexMessage{
		Body: &pb.PeerIndexMessage_SearchResponse_{
			SearchResponse: &pb.PeerIndexMessage_SearchResponse{
				RequestId: m.RequestId,
				Records:   records,
			},
		},
	}
	return sendProto(s.network, originHostID, PeerIndexPort, PeerIndexPort, msg)
}

func (s *peerIndex) handleSearchResponse(m *pb.PeerIndexMessage_SearchResponse) error {
	for _, r := range m.Records {
		if !verifyPeerIndexRecord(r) {
			continue
		}
		if !sendPeerIndexSearchResponse(&s.searchResponseChans, m.RequestId, r) {
			break
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

	rid, err := randUint64()
	if err != nil {
		return nil, err
	}

	msg := &pb.PeerIndexMessage{
		Body: &pb.PeerIndexMessage_SearchRequest_{
			SearchRequest: &pb.PeerIndexMessage_SearchRequest{
				RequestId: rid,
				Hash:      hash,
			},
		},
	}
	if err := sendProto(s.network, target, PeerIndexPort, PeerIndexPort, msg); err != nil {
		return nil, err
	}

	hosts := make(chan *PeerIndexHost, 32)
	s.searchResponseChans.Store(rid, hosts)
	go func() {
		<-ctx.Done()
		s.searchResponseChans.Delete(rid)
		close(hosts)
	}()

	return hosts, nil
}

func newPeerIndexItemKeyRange(peerIndexID uint32, hash []byte, localHostID kademlia.ID) (peerIndexItemKey, peerIndexItemKey) {
	min := newPeerIndexItemKey(peerIndexID, hash, localHostID, []byte{})
	max := newPeerIndexItemKey(peerIndexID+1, hash, localHostID, []byte{})
	return min, max
}

func newPeerIndexItemKey(peerIndexID uint32, hash []byte, localHostID kademlia.ID, remoteHostID []byte) (k peerIndexItemKey) {
	localHostID.Bytes(k[:])
	for i := 0; i < len(k) && i < len(hash); i++ {
		k[i] ^= hash[i]
	}

	binary.BigEndian.PutUint32(k[20:], peerIndexID)
	copy(k[24:], remoteHostID)
	return k
}

type peerIndexItemKey [44]byte

func (k peerIndexItemKey) Less(o peerIndexItemKey) bool {
	return bytes.Compare(k[:], o[:]) == -1
}

func newPeerIndexItemRecordRange(peerIndexID uint32, hash []byte, localHostID kademlia.ID) (*peerIndexItem, *peerIndexItem) {
	min, max := newPeerIndexItemKeyRange(peerIndexID, hash, localHostID)
	return &peerIndexItem{key: min}, &peerIndexItem{key: max}
}

func newPeerIndexItem(peerIndexID uint32, localHostID kademlia.ID, r *pb.PeerIndexMessage_Record) (*peerIndexItem, error) {
	hostID, err := kademlia.UnmarshalID(r.HostId)
	if err != nil {
		return nil, err
	}

	return &peerIndexItem{
		key:    newPeerIndexItemKey(peerIndexID, r.Hash, localHostID, r.HostId),
		hostID: hostID,
		record: unsafe.Pointer(r),
	}, nil
}

type peerIndexItem struct {
	key    peerIndexItemKey
	hostID kademlia.ID
	record unsafe.Pointer
}

func (p *peerIndexItem) HostID() kademlia.ID {
	return p.hostID
}

func (p *peerIndexItem) SetRecord(r *pb.PeerIndexMessage_Record) {
	atomic.SwapPointer(&p.record, unsafe.Pointer(r))
}

func (p *peerIndexItem) Record() *pb.PeerIndexMessage_Record {
	return (*pb.PeerIndexMessage_Record)(atomic.LoadPointer(&p.record))
}

// Less implements llrb.Item
func (p *peerIndexItem) Less(oi llrb.Item) bool {
	o, ok := oi.(*peerIndexItem)
	return ok && p.key.Less(o.key)
}

// ID implements kademlia.Interface
func (p *peerIndexItem) ID() kademlia.ID {
	return p.hostID
}

// Deadline implements timeoutQueueItem
func (p *peerIndexItem) Deadline() time.Time {
	return time.Unix(p.Record().Timestamp, 0).Add(peerIndexMaxRecordAge)
}

// NewPeerIndexStore ...
func NewPeerIndexStore(ctx context.Context, logger *zap.Logger, hostID kademlia.ID) *PeerIndexStore {
	p := &PeerIndexStore{
		logger:       logger,
		hostID:       hostID,
		records:      llrb.New(),
		discardQueue: newTimeoutQueue(ctx, peerIndexDiscardInterval, peerIndexMaxRecordAge),
	}

	p.ticker = TickerFunc(ctx, peerIndexDiscardInterval, p.tick)

	return p
}

// PeerIndexStore ...
type PeerIndexStore struct {
	logger       *zap.Logger
	hostID       kademlia.ID
	lock         sync.Mutex
	records      *llrb.LLRB
	discardQueue *timeoutQueue
	ticker       *Ticker
}

func (p *PeerIndexStore) tick(t time.Time) {
	p.lock.Lock()
	defer p.lock.Unlock()

	for {
		item, ok := p.discardQueue.Pop().(*peerIndexItem)
		if !ok {
			return
		}
		p.records.Delete(item)
	}
}

// Insert ...
func (p *PeerIndexStore) Insert(peerIndexID uint32, r *pb.PeerIndexMessage_Record) error {
	p.lock.Lock()
	defer p.lock.Unlock()

	item, err := newPeerIndexItem(peerIndexID, p.hostID, r)
	if err != nil {
		return err
	}

	prev, ok := p.records.Get(item).(*peerIndexItem)
	if !ok {
		p.records.ReplaceOrInsert(item)
		p.discardQueue.Push(item)

		if p.records.Len() > peerIndexMaxSize {
			p.records.Delete(p.records.Max())
		}
		return nil
	}

	if prev.Record().Timestamp < r.Timestamp {
		p.logger.Debug(
			"inserting peer index item",
			logutil.ByteHex("key", r.Key),
			logutil.ByteHex("hostId", r.HostId),
			zap.Uint32("port", r.Port),
		)
		prev.SetRecord(r)
	}

	return nil
}

// Remove ...
func (p *PeerIndexStore) Remove(peerIndexID uint32, r *pb.PeerIndexMessage_Record) error {
	p.lock.Lock()
	defer p.lock.Unlock()

	item, err := newPeerIndexItem(peerIndexID, p.hostID, r)
	if err != nil {
		return err
	}

	prev, ok := p.records.Get(item).(*peerIndexItem)
	if ok && prev.Record().Timestamp < r.Timestamp {
		p.logger.Debug(
			"removing peer index item",
			logutil.ByteHex("key", r.Key),
			logutil.ByteHex("hostId", r.HostId),
			zap.Uint32("port", r.Port),
		)
		p.records.Delete(item)
	}

	return nil
}

// Closest ...
func (p *PeerIndexStore) Closest(peerIndexID uint32, hostID kademlia.ID, hash []byte) (records []*pb.PeerIndexMessage_Record) {
	p.lock.Lock()
	defer p.lock.Unlock()

	f := kademlia.NewFilter(hostID)
	defer f.Free()

	iter := func(i llrb.Item) bool {
		f.Push(i.(*peerIndexItem))
		return true
	}
	min, max := newPeerIndexItemRecordRange(peerIndexID, hash, p.hostID)
	p.records.AscendRange(min, max, iter)

	for i := 0; i < peerIndexSearchResponseSize; i++ {
		v, ok := f.Pop()
		if !ok {
			return
		}
		records = append(records, v.(*peerIndexItem).Record())
	}
	return
}

func peerIndexRecordHash(key, salt []byte) []byte {
	hash := sha1.New()
	hash.Write(key)
	hash.Write(salt)
	return hash.Sum(nil)
}

// PeerIndexHost ...
type PeerIndexHost struct {
	Timestamp time.Time
	HostID    kademlia.ID
	Port      uint16
}

func sendPeerIndexSearchResponse(chans *sync.Map, requestID uint64, r *pb.PeerIndexMessage_Record) bool {
	ch, ok := chans.Load(requestID)
	if !ok {
		return false
	}

	hostID, err := kademlia.UnmarshalID(r.HostId)
	if err != nil {
		return false
	}
	h := &PeerIndexHost{
		Timestamp: time.Unix(r.Timestamp, 0),
		HostID:    hostID,
		Port:      uint16(r.Port),
	}

	select {
	case ch.(chan *PeerIndexHost) <- h:
		return true
	default:
		return false
	}
}

func newPeerIndexPublisher(ctx context.Context, logger *zap.Logger, network *Network, key, salt []byte, port uint16) (*peerIndexPublisher, error) {
	hash := peerIndexRecordHash(key, salt)
	target, err := kademlia.UnmarshalID(hash)
	if err != nil {
		return nil, err
	}

	record := &pb.PeerIndexMessage_Record{
		Hash:   hash,
		Key:    network.host.Key().Public,
		HostId: network.host.ID().Bytes(nil),
		Port:   uint32(port),
	}

	p := &peerIndexPublisher{
		logger:  logger,
		record:  record,
		target:  target,
		network: network,
	}

	p.ticker = TickerFuncWithCleanup(ctx, peerIndexPublishInterval, p.publish, p.unpublish)

	return p, nil
}

type peerIndexPublisher struct {
	logger  *zap.Logger
	record  *pb.PeerIndexMessage_Record
	target  kademlia.ID
	network *Network
	ticker  *Ticker
}

func (p *peerIndexPublisher) update() error {
	p.record.Timestamp = time.Now().Unix()
	if err := signPeerIndexRecord(p.record, p.network.host.Key()); err != nil {
		return err
	}
	return nil
}

func (p *peerIndexPublisher) publish(t time.Time) {
	if err := p.update(); err != nil {
		return
	}

	msg := &pb.PeerIndexMessage{
		Body: &pb.PeerIndexMessage_Publish_{
			Publish: &pb.PeerIndexMessage_Publish{
				Record: p.record,
			},
		},
	}
	if err := sendProto(p.network, p.target, PeerIndexPort, PeerIndexPort, msg); err != nil {
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

	msg := &pb.PeerIndexMessage{
		Body: &pb.PeerIndexMessage_Unpublish_{
			Unpublish: &pb.PeerIndexMessage_Unpublish{
				Record: p.record,
			},
		},
	}
	if err := sendProto(p.network, p.target, PeerIndexPort, PeerIndexPort, msg); err != nil {
		p.logger.Debug(
			"error unpublishing peer index item",
			zap.Error(err),
		)
	}
}

type peerIndexMarshaler struct {
	*pb.PeerIndexMessage_Record
}

func (r peerIndexMarshaler) Size() int {
	return 12 + len(r.Hash) + len(r.Key) + len(r.HostId)
}

func (r peerIndexMarshaler) Marshal(b []byte) int {
	n := copy(b, r.Hash)
	n += copy(b[n:], r.Key)
	n += copy(b[n:], r.HostId)
	binary.BigEndian.PutUint32(b[n:], uint32(r.Port))
	n += 4
	binary.BigEndian.PutUint64(b[n:], uint64(r.Timestamp))
	n += 8
	return n
}

func signPeerIndexRecord(r *pb.PeerIndexMessage_Record, key *pb.Key) error {
	if key.Type != pb.KeyType_KEY_TYPE_ED25519 {
		return errors.New("unsupported key type")
	}

	m := peerIndexMarshaler{r}
	b := pool.Get(uint16(m.Size()))
	defer pool.Put(b)
	m.Marshal(b)

	r.Signature = ed25519.Sign(ed25519.PrivateKey(key.Private), b)
	return nil
}

func verifyPeerIndexRecord(r *pb.PeerIndexMessage_Record) bool {
	m := peerIndexMarshaler{r}
	b := pool.Get(uint16(m.Size()))
	defer pool.Put(b)
	m.Marshal(b)

	if len(r.Key) != ed25519.PublicKeySize {
		return false
	}
	return ed25519.Verify(r.Key, b, r.Signature)
}
