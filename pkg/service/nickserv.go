package service

import (
	"bytes"
	"context"
	"crypto/ed25519"
	"encoding/binary"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"sync"
	"sync/atomic"
	"time"
	"unsafe"

	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/pool"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/prefixstream"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

var defaultNameChangeQuota uint32 = 5

const clientTimeout = 10 * time.Second

// TODO: crud handlers
// TODO: dao helpers for storing nickservtokens
// TODO: sign/verify nickservtoken

func NewNickServ(svc *NetworkServices, cfg *pb.ServerConfig, salt []byte) (*NickServ, error) {
	key := &pb.Key{}
	if err := json.Unmarshal(cfg.Key, &key); err != nil {
		return nil, err
	}

	w, err := ppspp.NewWriter(ppspp.WriterOptions{
		// SwarmOptions: ppspp.NewDefaultSwarmOptions(),
		SwarmOptions: ppspp.SwarmOptions{
			LiveWindow: 1 << 10, // 1MB
			ChunkSize:  128,
		},
		Key: key,
	})
	if err != nil {
		return nil, err
	}

	svc.Swarms.OpenSwarm(w.Swarm())

	port, err := svc.Network.ReservePort()
	if err != nil {
		return nil, err
	}

	ctx, cancel := context.WithCancel(context.Background())
	newSwarmPeerManager(ctx, svc, getPeersGetter(ctx, svc, key.Public, salt))

	b, err := proto.Marshal(&pb.NetworkAddress{
		HostId: svc.Host.ID().Bytes(nil),
		Port:   uint32(port),
	})
	if err != nil {
		cancel()
		return nil, err
	}
	_, err = svc.HashTable.Set(ctx, key, salt, b)
	if err != nil {
		cancel()
		return nil, err
	}

	s := &NickServ{
		close:           cancel,
		swarm:           w.Swarm(),
		svc:             svc,
		w:               prefixstream.NewWriter(w),
		roles:           cfg.GetRoles(),
		tokenttl:        cfg.GetTokenTtl(),
		nameChangeQuota: cfg.GetNameChangeQuota(),
	}
	err = svc.Network.SetHandler(port, s)
	if err != nil {
		cancel()
		return nil, err
	}

	return s, nil
}

type NickServ struct {
	//	cfg             *pb.ServConfig
	close           context.CancelFunc
	swarm           *ppspp.Swarm
	closeOnce       sync.Once
	svc             *NetworkServices
	w               *prefixstream.Writer
	store           *NickServStore
	roles           []string
	tokenttl        uint64
	nameChangeQuota uint32 // smaller?
}

func (s *NickServ) Close() {
	s.closeOnce.Do(func() {
		s.close()
		s.svc.Swarms.CloseSwarm(s.swarm.ID())
	})
}

func (s *NickServ) HandleMessage(msg *vpn.Message) (forward bool, err error) {
	var m pb.NickServRPCCommand
	if err := proto.Unmarshal(msg.Body, &m); err != nil {
		return true, err
	}

	// TODO: verify `source_public_key`

	switch b := m.Body.(type) {
	case *pb.NickServRPCCommand_Create_:
		err = s.handleCreate(m.RequestId, m.SourcePublicKey, b.Create)
	case *pb.NickServRPCCommand_Retrieve_:
		err = s.handleRetrieve(b.Retrieve)
	case *pb.NickServRPCCommand_Update_:
		err = s.handleUpdate(b.Update)
	case *pb.NickServRPCCommand_Delete_:
		err = s.handleDelete(b.Delete)
	default:
		err = errors.New("unexpected message type")
	}

	return true, nil
}

func (s *NickServ) handleCreate(id uint64, key []byte, msg *pb.NickServRPCCommand_Create) error {
	now := time.Now()
	rid, err := generateSnowflake()
	if err != nil {
		return err
	}
	r := &pb.NickServRecord{
		Id:                       rid,
		Key:                      key,
		Name:                     msg.Nick,
		CreatedTimestamp:         uint64(now.Unix()),
		UpdatedTimestamp:         uint64(now.Unix()),
		RemainingNameChangeQuota: defaultNameChangeQuota,
		Roles:                    []string{},
	}
	//return s.store.Insert(r)
	_ = r
	return fmt.Errorf("unimplemented")
}

func (s *NickServ) handleRetrieve(msg *pb.NickServRPCCommand_Retrieve) error {
	return fmt.Errorf("unimplemented")
}

func (s *NickServ) handleUpdate(msg *pb.NickServRPCCommand_Update) error {
	return fmt.Errorf("unimplemented")
}

func (s *NickServ) handleDelete(msg *pb.NickServRPCCommand_Delete) error {
	return fmt.Errorf("unimplemented")
}

func NewNickServClient(svc *NetworkServices, key, salt []byte) (*NickServClient, error) {
	port, err := svc.Network.ReservePort()
	if err != nil {
		return nil, err
	}

	swarm, err := ppspp.NewSwarm(
		ppspp.NewSwarmID(key),
		// ppspp.NewDefaultSwarmOptions(),
		ppspp.SwarmOptions{
			LiveWindow: 1 << 10, // 1MB
			ChunkSize:  128,
		},
	)
	if err != nil {
		return nil, err
	}
	svc.Swarms.OpenSwarm(swarm)

	ctx, cancel := context.WithCancel(context.Background())

	err = svc.PeerIndex.Publish(ctx, key, salt, 0)
	if err != nil {
		svc.Swarms.CloseSwarm(swarm.ID())
		cancel()
		return nil, err
	}

	newSwarmPeerManager(ctx, svc, getPeersGetter(ctx, svc, key, salt))

	c := &NickServClient{
		ctx:       ctx,
		close:     cancel,
		svc:       svc,
		swarm:     swarm,
		addrReady: make(chan struct{}),
		port:      port,
	}

	go c.syncAddr(svc, key, salt)

	return nil, nil
}

type NickServClient struct {
	ctx           context.Context
	close         context.CancelFunc
	closeOnce     sync.Once
	svc           *NetworkServices
	responses     map[uint64]chan pb.NickServRPCCommand
	nextRequestID uint64
	swarm         *ppspp.Swarm
	addrReady     chan struct{}
	addr          atomic.Value
	port          uint16
}

func (c *NickServClient) syncAddr(svc *NetworkServices, key, salt []byte) {
	var nextTick time.Duration
	var closeOnce sync.Once
	for {
		select {
		case <-time.After(nextTick):
			addr, err := getHostAddr(c.ctx, svc, key, salt)
			if err != nil {
				nextTick = syncAddrRetryIvl
				continue
			}

			c.addr.Store(addr)
			closeOnce.Do(func() { close(c.addrReady) })

			nextTick = syncAddrRefreshIvl
		case <-c.ctx.Done():
			return
		}
	}
}

// Close ...
func (c *NickServClient) Close() {
	c.closeOnce.Do(func() {
		c.close()
		c.svc.Swarms.CloseSwarm(c.swarm.ID())
	})
}

// Send ...
func (c *NickServClient) Send(key string, body []byte) error {
	select {
	case <-c.addrReady:
	case <-c.ctx.Done():
	}
	if c.ctx.Err() != nil {
		return c.ctx.Err()
	}

	id, err := generateSnowflake()
	if err != nil {
		return err
	}

	b, err := proto.Marshal(&pb.NickServRPCResponse{
		RequestId: id,
	})
	if err != nil {
		return err
	}

	addr := c.addr.Load().(*hostAddr)
	return c.svc.Network.Send(addr.HostID, addr.Port, c.port, b)
}

func readNickServEvents(swarm *ppspp.Swarm, messages chan *pb.NickServRPCCommand) {
	r := prefixstream.NewReader(swarm.Reader())
	b := bytes.NewBuffer(nil)
	for {
		b.Reset()
		if _, err := io.Copy(b, r); err != nil {
			return
		}

		var msg pb.NickServRPCCommand
		if err := proto.Unmarshal(b.Bytes(), &msg); err != nil {
			continue
		}
	}
}

func (c *NickServClient) MessageHandler(msg *pb.NickServRPCCommand) {
	ch, ok := c.responses[msg.RequestId]
	if !ok {
		return
	}
	select {
	case ch <- *msg:
	default:
	}
}

func (c *NickServClient) registerResponseChan() uint64 {
	rid := c.nextRequestID
	c.nextRequestID++

	c.responses[rid] = make(chan pb.NickServRPCCommand)

	return rid
}

func (c *NickServClient) awaitResponse(ctx context.Context, rid uint64) (*pb.NickServRPCResponse, error) {
	defer delete(c.responses, rid)

	ctx, cancel := context.WithTimeout(ctx, clientTimeout)
	defer cancel()

	select {
	case res := <-c.responses[rid]:
		return res, nil
	case <-ctx.Done():
		return nil, ctx.Err()
	}
}

func (c *NickServClient) Command(ctx context.Context, msg *pb.NickServRPCCommand) (*pb.NickServRPCResponse, error) {
	rid := c.registerResponseChan()

	// send proto message

	res, err := c.awaitResponse(ctx, rid)
	if err != nil {
		return nil, err
	}
	return res.Body.(*pb.NickServRPCResponse), nil
}

type NickServStore struct {
	logger  *zap.Logger
	hostID  kademlia.ID
	lock    sync.Mutex
	records *llrb.LLRB
}

func newNickServItemKeyRange(nickServID uint32, hash []byte, localHostID kademlia.ID) (nickServItemKey, nickServItemKey) {
	min := newNickServItemKey(nickServID, hash, localHostID, []byte{})
	max := newNickServItemKey(nickServID+1, hash, localHostID, []byte{})
	return min, max
}

func newNickServItemKey(nickServID uint32, hash []byte, localHostID kademlia.ID, remoteHostID []byte) (k nickServItemKey) {
	localHostID.Bytes(k[:])
	for i := 0; i < len(k) && i < len(hash); i++ {
		k[i] ^= hash[i]
	}

	binary.BigEndian.PutUint32(k[20:], nickServID)
	copy(k[24:], remoteHostID)
	return k
}

type nickServItemKey [44]byte

func (k nickServItemKey) Less(o nickServItemKey) bool {
	return bytes.Compare(k[:], o[:]) == -1
}

func newNickServItemRecordRange(nickServID uint32, hash []byte, localHostID kademlia.ID) (*nickServItem, *nickServItem) {
	min, max := newNickServItemKeyRange(nickServID, hash, localHostID)
	return &nickServItem{key: min}, &nickServItem{key: max}
}

func newNickServItem(nickServID uint32, localHostID kademlia.ID, r *pb.NickServRecord) (*nickServItem, error) {
	hostID, err := kademlia.UnmarshalID(r.HostId)
	if err != nil {
		return nil, err
	}

	return &nickServItem{
		key:    newNickServItemKey(nickServID, r.Hash, localHostID, r.HostId),
		hostID: hostID,
		record: unsafe.Pointer(r),
	}, nil
}

type nickServItem struct {
	key    nickServItemKey
	hostID kademlia.ID
	record unsafe.Pointer
}

func (p *nickServItem) HostID() kademlia.ID {
	return p.hostID
}

func (p *nickServItem) SetRecord(r *pb.NickServRecord) {
	atomic.SwapPointer(&p.record, unsafe.Pointer(r))
}

func (p *nickServItem) Record() *pb.NickServRecord {
	return (*pb.NickServRecord)(atomic.LoadPointer(&p.record))
}

// Less implements llrb.Item
func (p *nickServItem) Less(oi llrb.Item) bool {
	o, ok := oi.(*nickServItem)
	return ok && p.key.Less(o.key)
}

// ID implements kademlia.Interface
func (p *nickServItem) ID() kademlia.ID {
	return p.hostID
}

func (s *NickServStore) Insert(nickServID uint32, r *pb.NickServRecord) error {
	s.lock.Lock()
	defer s.lock.Unlock()

	item, err := newNickServItem(nickServID, s.hostID, r)
	if err != nil {
		return err
	}

	prev, ok := s.records.Get(item).(*nickServItem)
	if !ok {
		s.records.ReplaceOrInsert(item)
	}

	// TODO: logging

	prev.SetRecord(r)
	return nil
}

func (s *NickServStore) Delete(nickServID uint32, r *pb.NickServRecord) error {
	s.lock.Lock()
	defer s.lock.Unlock()

	item, err := newNickServItem(nickServID, s.hostID, r)
	if err != nil {
		return err
	}

	_, ok := s.records.Get(item).(*nickServItem)
	if ok {
		// TODO: logging
		s.records.Delete(item)
	}

	return nil
}

func (s *NickServStore) Update(r *pb.NickServRecord) error {
	return fmt.Errorf("unimplemented")
}

func (s *NickServStore) Retrieve(nick string) error {
	return fmt.Errorf("unimplemented")
}

type nickServMarshaler struct {
	*pb.NickServToken
}

func (r nickServMarshaler) Size() int {
	// TODO: roles
	return 8 + len(r.Key) + len(r.Nick) + len(r.Signature)
}

func (r nickServMarshaler) Marshal(b []byte) int {
	n := copy(b, r.Key)
	n += copy(b[n:], r.Nick)
	binary.BigEndian.PutUint64(b[n:], r.ValidUntil)
	n += 8
	n += copy(b[n:], r.Signature)
	// TODO: marshal roles
	return n
}

func signNickServToken(r *pb.NickServToken, key *pb.Key) error {
	if key.Type != pb.KeyType_KEY_TYPE_ED25519 {
		return errors.New("unsupported key type")
	}

	m := nickServMarshaler{r}
	b := pool.Get(uint16(m.Size()))
	defer pool.Put(b)
	m.Marshal(*b)

	r.Signature = ed25519.Sign(ed25519.PrivateKey(key.Private), *b)
	return nil
}

func verifyNickServToken(r *pb.NickServToken) bool {
	m := nickServMarshaler{r}
	b := pool.Get(uint16(m.Size()))
	defer pool.Put(b)
	m.Marshal(*b)

	if len(r.Key) != ed25519.PublicKeySize {
		return false
	}
	return ed25519.Verify(r.Key, *b, r.Signature)
}

var nextSnowflakeID uint64

// generate a 53 bit locally unique id
func generateSnowflake() (uint64, error) {
	seconds := uint64(time.Since(time.Date(2020, 0, 0, 0, 0, 0, 0, time.UTC)) / time.Second)
	sequence := atomic.AddUint64(&nextSnowflakeID, 1) << 32
	return (seconds | sequence) & 0x1fffffffffffff, nil
}
