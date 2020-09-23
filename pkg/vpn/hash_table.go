package vpn

import (
	"bytes"
	"context"
	"crypto/ed25519"
	"crypto/sha256"
	"encoding/binary"
	"errors"
	"log"
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

const hashTableSetInterval = 10 * time.Minute
const hashTableDiscardInterval = time.Minute
const hashTableMaxRecordAge = 30 * time.Minute
const hashTableMaxSize = 5120

var nextHashTableID uint32

// HashTable ...
type HashTable interface {
	Set(ctx context.Context, key *pb.Key, salt, value []byte) (*HashTablePublisher, error)
	Get(ctx context.Context, key, salt []byte) (<-chan *HashTableValue, error)
	HandleMessage(msg *Message) (forward bool, err error)
}

// NewHashTable ...
func NewHashTable(logger *zap.Logger, n *Network, store *HashTableStore) HashTable {
	id := atomic.AddUint32(&nextHashTableID, 1)
	if id == 0 {
		panic("hash table id overflow")
	}

	return &hashTable{
		logger:  logger,
		id:      id,
		store:   store,
		network: n,
	}
}

type hashTable struct {
	logger              *zap.Logger
	id                  uint32
	store               *HashTableStore
	network             *Network
	searchResponseChans sync.Map
}

func (s *hashTable) HandleMessage(msg *Message) (forward bool, err error) {
	var m pb.HashTableMessage
	if err := proto.Unmarshal(msg.Body, &m); err != nil {
		return true, err
	}

	switch b := m.Body.(type) {
	case *pb.HashTableMessage_Publish_:
		err = s.handlePublish(b.Publish.Record)
	case *pb.HashTableMessage_Unpublish_:
		err = s.handleUnpublish(b.Unpublish.Record)
	case *pb.HashTableMessage_GetRequest_:
		originHostID := s.network.host.ID()
		if len(msg.Trailers) != 0 {
			originHostID = msg.Trailers[0].HostID
		}
		err = s.handleGetRequest(b.GetRequest, originHostID)
	case *pb.HashTableMessage_GetResponse_:
		err = s.handleGetResponse(b.GetResponse)
	default:
		err = errors.New("unexpected message type")
	}

	return true, err
}

func (s *hashTable) handlePublish(r *pb.HashTableMessage_Record) error {
	if !verifyHashTableRecord(r) {
		return errors.New("invalid record signature")
	}

	return s.store.Insert(s.id, r)
}

func (s *hashTable) handleUnpublish(r *pb.HashTableMessage_Record) error {
	if !verifyHashTableRecord(r) {
		return errors.New("invalid record signature")
	}

	return s.store.Remove(s.id, r)
}

func (s *hashTable) handleGetRequest(m *pb.HashTableMessage_GetRequest, originHostID kademlia.ID) error {
	record := s.store.Get(s.id, m.Hash)
	if record == nil {
		return nil
	}

	msg := &pb.HashTableMessage{
		Body: &pb.HashTableMessage_GetResponse_{
			GetResponse: &pb.HashTableMessage_GetResponse{
				RequestId: m.RequestId,
				Record:    record,
			},
		},
	}
	return sendProto(s.network, originHostID, HashTablePort, HashTablePort, msg)
}

func (s *hashTable) handleGetResponse(m *pb.HashTableMessage_GetResponse) error {
	if !verifyHashTableRecord(m.Record) {
		return nil
	}

	if err := s.store.Insert(s.id, m.Record); err != nil {
		return err
	}
	sendHashTableGetResponse(&s.searchResponseChans, m.RequestId, m.Record)
	return nil
}

func (s *hashTable) Set(ctx context.Context, key *pb.Key, salt, value []byte) (*HashTablePublisher, error) {
	return newHashTablePublisher(ctx, s.logger, s.network, key, salt, value)
}

func (s *hashTable) Get(ctx context.Context, key, salt []byte) (<-chan *HashTableValue, error) {
	hash := hashTableRecordHash(key, salt)
	target, err := kademlia.UnmarshalID(hash)
	if err != nil {
		return nil, err
	}

	rid, err := randUint64()
	if err != nil {
		return nil, err
	}

	msg := &pb.HashTableMessage{
		Body: &pb.HashTableMessage_GetRequest_{
			GetRequest: &pb.HashTableMessage_GetRequest{
				RequestId: rid,
				Hash:      hash,
			},
		},
	}
	if err := sendProto(s.network, target, HashTablePort, HashTablePort, msg); err != nil {
		return nil, err
	}

	values := make(chan *HashTableValue, 32)
	s.searchResponseChans.Store(rid, values)
	go func() {
		<-ctx.Done()
		s.searchResponseChans.Delete(rid)
		close(values)
	}()

	return values, nil
}

func newHashTableItemKey(hashTableID uint32, hash []byte, localHostID kademlia.ID) (k hashTableItemKey) {
	localHostID.Bytes(k[:])
	for i := 0; i < len(k) && i < len(hash); i++ {
		k[i] ^= hash[i]
	}

	binary.BigEndian.PutUint32(k[kademlia.IDLength:], hashTableID)
	return k
}

type hashTableItemKey [kademlia.IDLength + 4]byte

func (k hashTableItemKey) Less(o hashTableItemKey) bool {
	return bytes.Compare(k[:], o[:]) == -1
}

func newHashTableItem(hashTableID uint32, localHostID kademlia.ID, r *pb.HashTableMessage_Record) *hashTableItem {
	hash := hashTableRecordHash(r.Key, r.Salt)
	return &hashTableItem{
		key:    newHashTableItemKey(hashTableID, hash, localHostID),
		record: unsafe.Pointer(r),
	}
}

type hashTableItem struct {
	key    hashTableItemKey
	record unsafe.Pointer
}

func (p *hashTableItem) SetRecord(r *pb.HashTableMessage_Record) {
	atomic.SwapPointer(&p.record, unsafe.Pointer(r))
}

func (p *hashTableItem) Record() *pb.HashTableMessage_Record {
	return (*pb.HashTableMessage_Record)(atomic.LoadPointer(&p.record))
}

// Less implements llrb.Item
func (p *hashTableItem) Less(oi llrb.Item) bool {
	o, ok := oi.(*hashTableItem)
	return ok && p.key.Less(o.key)
}

// Deadline implements timeoutQueueItem
func (p *hashTableItem) Deadline() time.Time {
	return time.Unix(p.Record().Timestamp, 0).Add(hashTableMaxRecordAge)
}

// NewHashTableStore ...
func NewHashTableStore(ctx context.Context, logger *zap.Logger, hostID kademlia.ID) *HashTableStore {
	p := &HashTableStore{
		logger:       logger,
		hostID:       hostID,
		records:      llrb.New(),
		discardQueue: newTimeoutQueue(ctx, hashTableDiscardInterval, hashTableMaxRecordAge),
	}

	p.ticker = TickerFunc(ctx, hashTableDiscardInterval, p.tick)

	return p
}

// HashTableStore ...
type HashTableStore struct {
	logger       *zap.Logger
	hostID       kademlia.ID
	lock         sync.Mutex
	records      *llrb.LLRB
	discardQueue *timeoutQueue
	ticker       *Ticker
}

func (p *HashTableStore) tick(t time.Time) {
	p.lock.Lock()
	defer p.lock.Unlock()

	for {
		item, ok := p.discardQueue.Pop().(*hashTableItem)
		if !ok {
			return
		}
		p.records.Delete(item)
	}
}

// Insert ...
func (p *HashTableStore) Insert(hashTableID uint32, r *pb.HashTableMessage_Record) error {
	p.lock.Lock()
	defer p.lock.Unlock()

	item := newHashTableItem(hashTableID, p.hostID, r)

	prev, ok := p.records.Get(item).(*hashTableItem)
	if !ok {
		p.logger.Debug(
			"inserting hash table item",
			logutil.ByteHex("key", r.Key),
			logutil.ByteHex("salt", r.Salt),
		)

		p.records.ReplaceOrInsert(item)
		p.discardQueue.Push(item)

		if p.records.Len() > hashTableMaxSize {
			p.records.Delete(p.records.Max())
		}
		return nil
	}

	if prev.Record().Timestamp > r.Timestamp {
		return errors.New("new record older than existing message")
	}

	p.logger.Debug(
		"updating hash table item",
		logutil.ByteHex("key", r.Key),
		logutil.ByteHex("salt", r.Salt),
	)
	prev.SetRecord(r)

	return nil
}

// Remove ...
func (p *HashTableStore) Remove(hashTableID uint32, r *pb.HashTableMessage_Record) error {
	p.lock.Lock()
	defer p.lock.Unlock()

	item := newHashTableItem(hashTableID, p.hostID, r)
	prev, ok := p.records.Get(item).(*hashTableItem)
	if ok && prev.Record().Timestamp < r.Timestamp {
		p.logger.Debug(
			"removing hash table item",
			logutil.ByteHex("key", r.Key),
			logutil.ByteHex("salt", r.Salt),
		)
		p.records.Delete(item)
	}

	return nil
}

// Get ...
func (p *HashTableStore) Get(hashTableID uint32, hash []byte) *pb.HashTableMessage_Record {
	p.lock.Lock()
	defer p.lock.Unlock()

	item, ok := p.records.Get(&hashTableItem{key: newHashTableItemKey(hashTableID, hash, p.hostID)}).(*hashTableItem)
	if !ok {
		return nil
	}
	return item.Record()
}

func hashTableRecordHash(key, salt []byte) []byte {
	hash := sha256.New()
	if _, err := hash.Write(key); err != nil {
		log.Println(err)
	}
	if _, err := hash.Write(salt); err != nil {
		log.Println(err)
	}
	return hash.Sum(nil)
}

// HashTableValue ...
type HashTableValue struct {
	Timestamp time.Time
	Value     []byte
}

type hashTableGetReceiver struct {
	requestID uint64
	chans     *sync.Map
	hosts     chan *HashTableValue
	closeOnce sync.Once
}

func (p *hashTableGetReceiver) Values() <-chan *HashTableValue {
	return p.hosts
}

func sendHashTableGetResponse(chans *sync.Map, requestID uint64, r *pb.HashTableMessage_Record) bool {
	ch, ok := chans.Load(requestID)
	if !ok {
		return false
	}

	h := &HashTableValue{
		Timestamp: time.Unix(r.Timestamp, 0),
		Value:     r.Value,
	}

	select {
	case ch.(chan *HashTableValue) <- h:
		return true
	default:
		return false
	}
}

func newHashTablePublisher(ctx context.Context, logger *zap.Logger, network *Network, key *pb.Key, salt, value []byte) (*HashTablePublisher, error) {
	target, err := kademlia.UnmarshalID(hashTableRecordHash(key.Public, salt))
	if err != nil {
		return nil, err
	}

	record := &pb.HashTableMessage_Record{
		Key:   key.Public,
		Salt:  salt,
		Value: value,
	}

	ctx, cancel := context.WithCancel(ctx)

	p := &HashTablePublisher{
		logger:  logger,
		key:     key,
		record:  record,
		target:  target,
		network: network,
		close:   cancel,
	}

	p.ticker = TickerFuncWithCleanup(ctx, hashTableSetInterval, p.publish, p.unpublish)

	return p, nil
}

// HashTablePublisher ...
type HashTablePublisher struct {
	logger  *zap.Logger
	lock    sync.Mutex
	key     *pb.Key
	record  *pb.HashTableMessage_Record
	target  kademlia.ID
	network *Network
	close   context.CancelFunc
	ticker  *Ticker
}

// Update ...
func (p *HashTablePublisher) Update(v []byte) {
	p.lock.Lock()
	defer p.lock.Unlock()
	p.record.Value = v
}

// Close ...
func (p *HashTablePublisher) Close() {
	p.close()
}

func (p *HashTablePublisher) update() error {
	p.record.Timestamp = time.Now().Unix()
	return signHashTableRecord(p.record, p.key)
}

func (p *HashTablePublisher) publish(t time.Time) {
	p.lock.Lock()
	defer p.lock.Unlock()

	if err := p.update(); err != nil {
		return
	}

	msg := &pb.HashTableMessage{
		Body: &pb.HashTableMessage_Publish_{
			Publish: &pb.HashTableMessage_Publish{
				Record: p.record,
			},
		},
	}
	if err := sendProto(p.network, p.target, HashTablePort, HashTablePort, msg); err != nil {
		p.logger.Debug(
			"error publishing hash table item",
			zap.Error(err),
		)
	}
}

func (p *HashTablePublisher) unpublish() {
	p.lock.Lock()
	defer p.lock.Unlock()

	if err := p.update(); err != nil {
		return
	}

	msg := &pb.HashTableMessage{
		Body: &pb.HashTableMessage_Unpublish_{
			Unpublish: &pb.HashTableMessage_Unpublish{
				Record: p.record,
			},
		},
	}
	if err := sendProto(p.network, p.target, HashTablePort, HashTablePort, msg); err != nil {
		p.logger.Debug(
			"error unpublishing hash table item",
			zap.Error(err),
		)
	}
}

type hashTableRecordMarshaler struct {
	*pb.HashTableMessage_Record
}

func (r hashTableRecordMarshaler) Size() int {
	return 8 + len(r.Key) + len(r.Salt) + len(r.Value)
}

func (r hashTableRecordMarshaler) Marshal(b []byte) int {
	n := copy(b, r.Key)
	n += copy(b[n:], r.Salt)
	n += copy(b[n:], r.Value)
	binary.BigEndian.PutUint64(b[n:], uint64(r.Timestamp))
	n += 8
	return n
}

func signHashTableRecord(r *pb.HashTableMessage_Record, key *pb.Key) error {
	if key.Type != pb.KeyType_KEY_TYPE_ED25519 {
		return errors.New("unsupported key type")
	}

	m := hashTableRecordMarshaler{r}
	b := pool.Get(uint16(m.Size()))
	defer pool.Put(b)
	m.Marshal(*b)

	r.Signature = ed25519.Sign(ed25519.PrivateKey(key.Private), *b)
	return nil
}

func verifyHashTableRecord(r *pb.HashTableMessage_Record) bool {
	m := hashTableRecordMarshaler{r}
	b := pool.Get(uint16(m.Size()))
	defer pool.Put(b)
	m.Marshal(*b)

	if len(r.Key) != ed25519.PublicKeySize {
		return false
	}
	return ed25519.Verify(r.Key, *b, r.Signature)
}
