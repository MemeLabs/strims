package vpn

import (
	"bytes"
	"crypto/sha1"
	"encoding/binary"
	"errors"
	"runtime"
	"sync"
	"sync/atomic"
	"time"
	"unsafe"

	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/petar/GoLLRB/llrb"
	"google.golang.org/protobuf/proto"
)

const peerIndexSearchResponseSize = 5
const peerIndexPublishInterval = 10 * time.Second
const peerIndexDiscardInterval = time.Second
const peerIndexMaxRecordAge = 30 * time.Second
const peerIndexMaxSize = 5120

var nextPeerIndexID uint32

// PeerIndex ...
type PeerIndex interface {
	Publish(key, salt []byte, port uint16) (PeerIndexPublisher, error)
	Search(key, salt []byte) (PeerIndexSearchReceiver, error)
	HandleMessage(msg *Message) (forward bool, err error)
}

func NewPeerIndex(n *Network, store *PeerIndexStore) PeerIndex {
	id := atomic.AddUint32(&nextPeerIndexID, 1)
	if id == 0 {
		panic("peer index id overflow")
	}

	return &peerIndex{
		id:      id,
		store:   store,
		network: n,
	}
}

type peerIndex struct {
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
	if !verifyProto(r) {
		return errors.New("invalid record signature")
	}

	return s.store.Insert(s.id, r)
}

func (s *peerIndex) handleUnpublish(r *pb.PeerIndexMessage_Record) error {
	if !verifyProto(r) {
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
		if !verifyProto(r) {
			continue
		}
		if !sendPeerIndexSearchResponse(&s.searchResponseChans, m.RequestId, r) {
			break
		}
	}

	return nil
}

func (s *peerIndex) Publish(key, salt []byte, port uint16) (PeerIndexPublisher, error) {
	return newPeerIndexPublisher(s.network, key, salt, port)
}

func (s *peerIndex) Search(key, salt []byte) (PeerIndexSearchReceiver, error) {
	hash := peerIndexRecordHash(key, salt)
	target, err := kademlia.UnmarshalID(hash)
	if err != nil {
		return nil, err
	}

	rid, err := randUint64()
	if err != nil {
		return nil, err
	}
	r := newPeerIndexSearchReceiver(&s.searchResponseChans, rid)

	msg := &pb.PeerIndexMessage{
		Body: &pb.PeerIndexMessage_SearchRequest_{
			SearchRequest: &pb.PeerIndexMessage_SearchRequest{
				RequestId: rid,
				Hash:      hash,
			},
		},
	}
	if err := sendProto(s.network, target, PeerIndexPort, PeerIndexPort, msg); err != nil {
		r.Close()
		return nil, err
	}

	return r, nil
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

// Deadline implements discardQueueItem
func (p *peerIndexItem) Deadline() time.Time {
	return time.Unix(p.Record().Timestamp, 0).Add(peerIndexMaxRecordAge)
}

func NewPeerIndexStore(hostID kademlia.ID) *PeerIndexStore {
	p := &PeerIndexStore{
		hostID:       hostID,
		records:      llrb.New(),
		discardQueue: newDiscardQueue(peerIndexDiscardInterval, peerIndexMaxRecordAge),
	}

	p.Poller = NewPoller(peerIndexDiscardInterval, p.tick, p.discardQueue.Stop)

	return p
}

type PeerIndexStore struct {
	hostID       kademlia.ID
	lock         sync.Mutex
	records      *llrb.LLRB
	discardQueue *discardQueue
	*Poller
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
		prev.SetRecord(r)
	}

	return nil
}

func (p *PeerIndexStore) Remove(peerIndexID uint32, r *pb.PeerIndexMessage_Record) error {
	p.lock.Lock()
	defer p.lock.Unlock()

	item, err := newPeerIndexItem(peerIndexID, p.hostID, r)
	if err != nil {
		return err
	}

	prev, ok := p.records.Get(item).(*peerIndexItem)
	if ok && prev.Record().Timestamp < r.Timestamp {
		p.records.Delete(item)
	}

	return nil
}

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

// PeerIndexSearchReceiver ...
type PeerIndexSearchReceiver interface {
	Hosts() <-chan *PeerIndexHost
	SetTimeout(d time.Duration)
	Close()
}

func newPeerIndexSearchReceiver(chans *sync.Map, requestID uint64) *peerIndexSearchReceiver {
	hosts := make(chan *PeerIndexHost, 32)
	chans.Store(requestID, hosts)

	p := &peerIndexSearchReceiver{
		requestID: requestID,
		chans:     chans,
		hosts:     hosts,
	}

	runtime.SetFinalizer(p, func(r *peerIndexSearchReceiver) { r.Close() })

	return p
}

type peerIndexSearchReceiver struct {
	requestID uint64
	chans     *sync.Map
	hosts     chan *PeerIndexHost
	closeOnce sync.Once
}

func (p *peerIndexSearchReceiver) Hosts() <-chan *PeerIndexHost {
	return p.hosts
}

func (p *peerIndexSearchReceiver) SetTimeout(d time.Duration) {
	go func() {
		time.Sleep(d)
		p.Close()
	}()
}

func (p *peerIndexSearchReceiver) Close() {
	p.closeOnce.Do(func() {
		close(p.hosts)
		p.chans.Delete(p.requestID)
	})
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

// PeerIndexPublisher ...
type PeerIndexPublisher interface {
	Stop()
}

func newPeerIndexPublisher(network *Network, key, salt []byte, port uint16) (*peerIndexPublisher, error) {
	hash := peerIndexRecordHash(key, salt)
	target, err := kademlia.UnmarshalID(hash)
	if err != nil {
		return nil, err
	}

	record := &pb.PeerIndexMessage_Record{
		Hash:   hash,
		HostId: network.host.ID().Bytes(nil),
		Port:   uint32(port),
	}

	p := &peerIndexPublisher{
		record:  record,
		target:  target,
		network: network,
	}

	p.Poller = NewPoller(peerIndexPublishInterval, p.publish, p.unpublish)
	runtime.SetFinalizer(p, func(p *peerIndexPublisher) { p.Stop() })

	return p, nil
}

type peerIndexPublisher struct {
	record  *pb.PeerIndexMessage_Record
	target  kademlia.ID
	network *Network
	*Poller
}

func (p *peerIndexPublisher) update() error {
	p.record.Timestamp = time.Now().Unix()
	if err := signProto(p.record, p.network.host.Key()); err != nil {
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
	sendProto(p.network, p.target, PeerIndexPort, PeerIndexPort, msg)
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
	sendProto(p.network, p.target, PeerIndexPort, PeerIndexPort, msg)
}
