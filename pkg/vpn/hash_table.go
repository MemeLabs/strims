package vpn

import (
	"bytes"
	"context"
	"crypto/sha256"
	"encoding/binary"
	"errors"
	"log"
	"sync"
	"sync/atomic"
	"time"
	"unsafe"

	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	vpnv1 "github.com/MemeLabs/go-ppspp/pkg/apis/vpn/v1"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/randutil"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

const hashTableSetInterval = 10 * time.Minute
const hashTableDiscardInterval = time.Minute
const hashTableMaxRecordAge = 30 * time.Minute
const hashTableMaxSize = 5120
const hashTableGetTimeout = time.Minute

// HashTable ...
type HashTable interface {
	Set(ctx context.Context, key *key.Key, salt, value []byte) (*HashTablePublisher, error)
	Get(ctx context.Context, key, salt []byte, options ...HashTableOption) (<-chan []byte, error)
}

func newHashTable(logger *zap.Logger, n *Network, store *HashTableStore) *hashTable {
	return &hashTable{
		logger:  logger,
		store:   store.accessor(),
		network: n,
	}
}

// HashTable ...
type hashTable struct {
	logger              *zap.Logger
	store               *hashTableStoreAccesstor
	network             *Network
	searchResponseChans sync.Map
}

// HandleMessage ...
func (s *hashTable) HandleMessage(msg *Message) error {
	if msg.Trailer.Hops == 0 {
		return nil
	}

	var m vpnv1.HashTableMessage
	if err := proto.Unmarshal(msg.Body, &m); err != nil {
		return err
	}

	switch b := m.Body.(type) {
	case *vpnv1.HashTableMessage_Publish_:
		return s.handlePublish(b.Publish.Record)
	case *vpnv1.HashTableMessage_Unpublish_:
		return s.handleUnpublish(b.Unpublish.Record)
	case *vpnv1.HashTableMessage_GetRequest_:
		return s.handleGetRequest(b.GetRequest, msg.SrcHostID())
	case *vpnv1.HashTableMessage_GetResponse_:
		return s.handleGetResponse(b.GetResponse, msg.Header.DstID)
	default:
		return errors.New("unexpected message type")
	}
}

func (s *hashTable) handlePublish(r *vpnv1.HashTableMessage_Record) error {
	if err := dao.VerifyMessage(r); err != nil {
		return err
	}

	s.store.upsert(r)
	return nil
}

func (s *hashTable) handleUnpublish(r *vpnv1.HashTableMessage_Record) error {
	if err := dao.VerifyMessage(r); err != nil {
		return err
	}

	return s.store.remove(r)
}

func (s *hashTable) handleGetRequest(m *vpnv1.HashTableMessage_GetRequest, originHostID kademlia.ID) error {
	record := s.store.get(m.Hash)
	if record == nil || record.Timestamp <= m.IfModifiedSince {
		return nil
	}

	msg := &vpnv1.HashTableMessage{
		Body: &vpnv1.HashTableMessage_GetResponse_{
			GetResponse: &vpnv1.HashTableMessage_GetResponse{
				RequestId: m.RequestId,
				Record:    record,
			},
		},
	}
	return s.network.SendProto(originHostID, vnic.HashTablePort, vnic.HashTablePort, msg)
}

func (s *hashTable) handleGetResponse(m *vpnv1.HashTableMessage_GetResponse, target kademlia.ID) error {
	if dao.VerifyMessage(m.Record) != nil {
		return nil
	}

	if !s.store.upsert(m.Record) {
		return nil
	}

	if !s.network.host.ID().Equals(target) {
		return nil
	}

	if ch, ok := s.searchResponseChans.Load(m.RequestId); ok {
		select {
		case ch.(chan []byte) <- m.Record.Value:
		default:
		}
	}

	return nil
}

// Set ...
func (s *hashTable) Set(ctx context.Context, key *key.Key, salt, value []byte) (*HashTablePublisher, error) {
	return newHashTablePublisher(ctx, s.logger, s.network, s.store, key, salt, value)
}

type hashTableGetOptions struct {
	disableCache bool
	cacheOnly    bool
	timeout      time.Duration
}

// HashTableOption ...
type HashTableOption func(*hashTableGetOptions)

// DisableCache ...
func DisableCache() func(*hashTableGetOptions) {
	return func(opts *hashTableGetOptions) {
		opts.disableCache = true
	}
}

// WithTimeout ...
func WithTimeout(d time.Duration) func(*hashTableGetOptions) {
	return func(opts *hashTableGetOptions) {
		opts.timeout = d
	}
}

// Get ...
func (s *hashTable) Get(ctx context.Context, key, salt []byte, options ...HashTableOption) (<-chan []byte, error) {
	opts := &hashTableGetOptions{
		timeout: hashTableGetTimeout,
	}
	for _, opt := range options {
		opt(opts)
	}

	hash := hashTableRecordHash(key, salt)
	target, err := kademlia.UnmarshalID(hash)
	if err != nil {
		return nil, err
	}

	rid, err := randutil.Uint64()
	if err != nil {
		return nil, err
	}

	var timestamp int64
	values := make(chan []byte, 32)

	if !opts.disableCache {
		if record := s.store.get(hash); record != nil {
			timestamp = record.Timestamp
			values <- record.Value
		}
	}

	s.searchResponseChans.Store(rid, values)
	cleanup := func() {
		s.searchResponseChans.Delete(rid)
		close(values)
	}

	if opts.disableCache || timeutil.Now().After(timeutil.Unix(timestamp, 0).Add(hashTableSetInterval)) {
		msg := &vpnv1.HashTableMessage{
			Body: &vpnv1.HashTableMessage_GetRequest_{
				GetRequest: &vpnv1.HashTableMessage_GetRequest{
					RequestId:       rid,
					Hash:            hash,
					IfModifiedSince: timestamp,
				},
			},
		}
		if err := s.network.SendProto(target, vnic.HashTablePort, vnic.HashTablePort, msg); err != nil {
			cleanup()
			return nil, err
		}
	}

	ctx, cancel := context.WithTimeout(ctx, opts.timeout)
	go func() {
		<-ctx.Done()
		cancel()
		cleanup()
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

func newHashTableItem(hashTableID uint32, localHostID kademlia.ID, r *vpnv1.HashTableMessage_Record) *hashTableItem {
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

func (p *hashTableItem) SetRecord(r *vpnv1.HashTableMessage_Record) {
	atomic.SwapPointer(&p.record, unsafe.Pointer(r))
}

func (p *hashTableItem) Record() *vpnv1.HashTableMessage_Record {
	return (*vpnv1.HashTableMessage_Record)(atomic.LoadPointer(&p.record))
}

// Less implements llrb.Item
func (p *hashTableItem) Less(oi llrb.Item) bool {
	o, ok := oi.(*hashTableItem)
	return ok && p.key.Less(o.key)
}

// Deadline implements timeoutQueueItem
func (p *hashTableItem) Deadline() timeutil.Time {
	return timeutil.Unix(p.Record().Timestamp, 0).Add(hashTableMaxRecordAge)
}

// newHashTableStore ...
func newHashTableStore(ctx context.Context, logger *zap.Logger, hostID kademlia.ID) *HashTableStore {
	p := &HashTableStore{
		logger:       logger,
		hostID:       hostID,
		records:      llrb.New(),
		discardQueue: newTimeoutQueue(ctx, hashTableDiscardInterval, hashTableMaxRecordAge),
	}

	p.ticker = timeutil.TickerBFunc(ctx, hashTableDiscardInterval, p.tick)

	return p
}

// HashTableStore ...
type HashTableStore struct {
	logger         *zap.Logger
	hostID         kademlia.ID
	lock           sync.Mutex
	records        *llrb.LLRB
	discardQueue   *timeoutQueue
	ticker         *timeutil.TickerB
	nextAccessorID uint32
}

func (p *HashTableStore) accessor() *hashTableStoreAccesstor {
	id := atomic.AddUint32(&p.nextAccessorID, 1)
	if id == 0 {
		panic("hash table id overflow")
	}

	return &hashTableStoreAccesstor{
		id:    id,
		store: p,
	}
}

func (p *HashTableStore) tick(t timeutil.Time) {
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

func (p *HashTableStore) upsert(hashTableID uint32, r *vpnv1.HashTableMessage_Record) bool {
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
		return true
	}

	if prev.Record().Timestamp > r.Timestamp {
		return false
	}

	p.logger.Debug(
		"updating hash table item",
		logutil.ByteHex("key", r.Key),
		logutil.ByteHex("salt", r.Salt),
	)

	modified := bytes.Equal(prev.Record().Value, r.Value)
	prev.SetRecord(r)

	return modified
}

func (p *HashTableStore) remove(hashTableID uint32, r *vpnv1.HashTableMessage_Record) error {
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
func (p *HashTableStore) get(hashTableID uint32, hash []byte) *vpnv1.HashTableMessage_Record {
	p.lock.Lock()
	defer p.lock.Unlock()

	item, ok := p.records.Get(&hashTableItem{key: newHashTableItemKey(hashTableID, hash, p.hostID)}).(*hashTableItem)
	if !ok {
		return nil
	}
	return item.Record()
}

type hashTableStoreAccesstor struct {
	id    uint32
	store *HashTableStore
}

func (p *hashTableStoreAccesstor) upsert(r *vpnv1.HashTableMessage_Record) bool {
	return p.store.upsert(p.id, r)
}

func (p *hashTableStoreAccesstor) remove(r *vpnv1.HashTableMessage_Record) error {
	return p.store.remove(p.id, r)
}

func (p *hashTableStoreAccesstor) get(hash []byte) *vpnv1.HashTableMessage_Record {
	return p.store.get(p.id, hash)
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

type hashTableGetReceiver struct {
	requestID uint64
	chans     *sync.Map
	values    chan []byte
	closeOnce sync.Once
}

func (p *hashTableGetReceiver) Values() <-chan []byte {
	return p.values
}

func sendHashTableGetResponse(chans *sync.Map, requestID uint64, v []byte) bool {
	ch, ok := chans.Load(requestID)
	if !ok {
		return false
	}

	select {
	case ch.(chan []byte) <- v:
		return true
	default:
		return false
	}
}

func newHashTablePublisher(ctx context.Context, logger *zap.Logger, network *Network, store *hashTableStoreAccesstor, key *key.Key, salt, value []byte) (*HashTablePublisher, error) {
	target, err := kademlia.UnmarshalID(hashTableRecordHash(key.Public, salt))
	if err != nil {
		return nil, err
	}

	record := &vpnv1.HashTableMessage_Record{
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
		store:   store,
		close:   cancel,
	}

	p.ticker = timeutil.TickerBFuncWithCleanup(ctx, hashTableSetInterval, p.publish, p.unpublish)

	return p, nil
}

// HashTablePublisher ...
type HashTablePublisher struct {
	logger  *zap.Logger
	lock    sync.Mutex
	key     *key.Key
	record  *vpnv1.HashTableMessage_Record
	target  kademlia.ID
	network *Network
	store   *hashTableStoreAccesstor
	close   context.CancelFunc
	ticker  *timeutil.TickerB
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
	p.record.Timestamp = timeutil.Now().Unix()
	return dao.SignMessage(p.record, p.key)
}

func (p *HashTablePublisher) publish(t timeutil.Time) {
	p.lock.Lock()
	defer p.lock.Unlock()

	if err := p.update(); err != nil {
		return
	}

	p.store.upsert(p.record)

	msg := &vpnv1.HashTableMessage{
		Body: &vpnv1.HashTableMessage_Publish_{
			Publish: &vpnv1.HashTableMessage_Publish{
				Record: p.record,
			},
		},
	}
	if err := p.network.SendProto(p.target, vnic.HashTablePort, vnic.HashTablePort, msg); err != nil {
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

	msg := &vpnv1.HashTableMessage{
		Body: &vpnv1.HashTableMessage_Unpublish_{
			Unpublish: &vpnv1.HashTableMessage_Unpublish{
				Record: p.record,
			},
		},
	}
	if err := p.network.SendProto(p.target, vnic.HashTablePort, vnic.HashTablePort, msg); err != nil {
		p.logger.Debug(
			"error unpublishing hash table item",
			zap.Error(err),
		)
	}
}
