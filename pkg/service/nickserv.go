package service

import (
	"bytes"
	"context"
	"crypto/ed25519"
	"encoding/binary"
	"errors"
	"fmt"
	"reflect"
	"sort"
	"strings"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

var (
	defaultNameChangeQuota uint32 = 5

	ErrNotFound             = errors.New("Could not find item matching criteria")
	ErrNameChangesExhausted = errors.New("no name changes remaining")
	ErrNameAlreadyTaken     = errors.New("the requested name is already in use")
	ErrUnimplemented        = errors.New("not implemented")
	ErrAlreadyExists        = errors.New("a record for this key already exists")
	ErrRoleNotExist         = errors.New("the requested role does not exist")
)

const clientTimeout = 10 * time.Second

// TODO: tests
// TODO: get profile store from frontend service

func NewNickServ(svc *NetworkServices, salt []byte, kvStore kv.RWStore, nickservID uint64) (*NickServ, error) {
	cfg, err := dao.GetNickservConfig(kvStore, nickservID) // TODO: nickserv id
	if err != nil {
		return nil, err
	}

	port, err := svc.Network.ReservePort()
	if err != nil {
		return nil, err
	}

	b, err := proto.Marshal(&pb.NetworkAddress{
		HostId: svc.Host.ID().Bytes(nil),
		Port:   uint32(port),
	})
	if err != nil {
		return nil, err
	}

	ctx, cancel := context.WithCancel(context.Background())
	_, err = svc.HashTable.Set(ctx, cfg.Key, salt, b)
	if err != nil {
		cancel()
		return nil, err
	}

	rolesMap := make(map[string]bool)
	for _, role := range cfg.Roles {
		rolesMap[role] = true
	}

	store, err := NewNickServStore(kvStore, nickservID)
	if err != nil {
		cancel()
		return nil, err
	}

	s := &NickServ{
		logger:          svc.Host.Logger(),
		close:           cancel,
		svc:             svc,
		roles:           rolesMap,
		tokenTTL:        cfg.TokenTtl.AsDuration(),
		nameChangeQuota: cfg.NameChangeQuota,
		store:           store,
	}

	s.logger.Info("started nickerv")

	err = svc.Network.SetHandler(port, s)
	if err != nil {
		cancel()
		return nil, err
	}

	return s, nil
}

type NickServ struct {
	logger          *zap.Logger
	close           context.CancelFunc
	closeOnce       sync.Once
	svc             *NetworkServices
	store           *NickServStore
	roles           map[string]bool
	tokenTTL        time.Duration
	nameChangeQuota uint32 // smaller?
}

func (s *NickServ) Close() {
	s.closeOnce.Do(func() {
		s.logger.Info("stopping nickserv")
		s.close()
	})
}

func (s *NickServ) HandleMessage(msg *vpn.Message) (forward bool, err error) {
	var m pb.NickServRPCCommand
	if err := proto.Unmarshal(msg.Body, &m); err != nil {
		return true, err
	}

	valid := ed25519.Verify(m.SourcePublicKey, msg.Body, msg.Trailers[0].Signature)
	if !valid {
		s.logger.Warn("failed to verify message signature",
			zap.Uint64("requestID", m.RequestId),
			zap.Binary("publicKey", m.SourcePublicKey),
		)
		return false, dao.ErrInvalidSignature
	}

	var resp proto.Message

	switch b := m.Body.(type) {
	case *pb.NickServRPCCommand_Create_:
		resp, err = s.handleCreate(m.SourcePublicKey, b.Create)
	case *pb.NickServRPCCommand_Retrieve_:
		resp, err = s.handleRetrieve(m.SourcePublicKey)
	case *pb.NickServRPCCommand_Update_:
		resp, err = s.handleUpdate(m.SourcePublicKey, b.Update)
	case *pb.NickServRPCCommand_Delete_:
		resp, err = s.handleDelete(m.SourcePublicKey)
	default:
		err = errors.New("unexpected message type")
	}

	if err != nil {
		s.logger.Error("failed to handle nickerv message",
			zap.Uint64("requestID", m.RequestId),
			zap.String("requestType", reflect.TypeOf(m.Body).Name()),
			zap.Error(err),
		)

		resp = &pb.NickServRPCResponse{
			Body: &pb.NickServRPCResponse_Error{
				Error: err.Error(),
			},
		}
	}

	// TODO: return some errors that can occur during handling

	return false, s.Send(resp, msg.Trailers[0].HostID, msg.Header.SrcPort, msg.Header.DstPort)
}

func (s *NickServ) Send(msg proto.Message, dstID kademlia.ID, dstPort, srcPort uint16) error {
	msgBytes, err := proto.Marshal(msg)
	if err != nil {
		return fmt.Errorf("failed to marshal message: %w", err)
	}

	return s.svc.Network.Send(dstID, dstPort, srcPort, msgBytes)
}

func (s *NickServ) handleCreate(key []byte, msg *pb.NickServRPCCommand_Create) (*pb.NickServRPCResponse, error) {
	r := &pb.NickservNick{
		Key:                      key,
		Nick:                     msg.Nick,
		RemainingNameChangeQuota: defaultNameChangeQuota,
		Roles:                    []string{},
	}

	r, err := s.store.Insert(r)
	if err != nil {
		return nil, err
	}

	token, err := s.toSignedToken(r)
	if err != nil {
		return nil, err
	}

	return &pb.NickServRPCResponse{
		Body: &pb.NickServRPCResponse_Create{
			Create: token,
		},
	}, nil
}

func (s *NickServ) toSignedToken(r *pb.NickservNick) (*pb.NickServToken, error) {
	token := &pb.NickServToken{
		Key:        r.Key,
		Nick:       r.Nick,
		Roles:      []string{}, // TODO
		ValidUntil: s.validUntil(),
	}

	s.logger.Debug("retrieved nick token",
		zap.Uint64("id", r.Id),
		zap.Binary("publicKey", r.Key),
		zap.String("nick", r.Nick),
		zap.Strings("roles", r.Roles),
		zap.Uint64("validUntil", token.ValidUntil),
	)

	err := signNickToken(token, s.svc.Host.Key())
	return token, err
}

func (s *NickServ) handleRetrieve(key []byte) (*pb.NickServRPCResponse, error) {
	record, err := s.store.Retrieve(key)
	if err != nil {
		return nil, err
	}

	token, err := s.toSignedToken(record)
	if err != nil {
		return nil, err
	}

	return &pb.NickServRPCResponse{
		Body: &pb.NickServRPCResponse_Retrieve{
			Retrieve: token,
		},
	}, nil
}

func (s *NickServ) handleUpdate(key []byte, msg *pb.NickServRPCCommand_Update) (*pb.NickServRPCResponse, error) {
	record, err := s.store.Retrieve(key)
	if err != nil {
		return nil, err
	}

	switch t := msg.Param.(type) {
	case *pb.NickServRPCCommand_Update_NameChangeQuota:
		// TODO handle permissions for quota changes
		return nil, ErrUnimplemented
	case *pb.NickServRPCCommand_Update_Nick:
		if record.RemainingNameChangeQuota <= 0 {
			return nil, ErrNameChangesExhausted
		}

		if record.Nick != t.Nick.OldNick {
			return nil, errors.New("nick doesn't match record")
		}

		s.logger.Info("updating user nick",
			zap.Uint64("id", record.Id),
			zap.Binary("publicKey", record.Key),
			zap.String("oldNick", t.Nick.OldNick),
			zap.String("newNick", t.Nick.NewNick),
			zap.Uint32("remainingNameChangeQuota", record.RemainingNameChangeQuota),
		)

		record.Nick = t.Nick.NewNick
		record.RemainingNameChangeQuota--
		_, err = s.store.Update(record)
		if err != nil {
			return nil, err
		}
	case *pb.NickServRPCCommand_Update_Roles_:
		for _, role := range t.Roles.Roles {
			if _, ok := s.roles[role]; !ok {
				return nil, ErrRoleNotExist
			}
		}

		// TODO handle permissions for role changes
		return nil, ErrUnimplemented
	}

	return &pb.NickServRPCResponse{
		Body: &pb.NickServRPCResponse_Update_{
			Update: &pb.NickServRPCResponse_Update{},
		},
	}, nil
}

func (s *NickServ) handleDelete(key []byte) (*pb.NickServRPCResponse, error) {
	record, err := s.store.Retrieve(key)
	if err == nil {
		return nil, err
	}

	s.logger.Info("deleting user nick",
		zap.Uint64("id", record.Id),
		zap.Binary("publicKey", record.Key),
		zap.String("nick", record.Nick),
		zap.Strings("roles", record.Roles),
	)

	err = s.store.Delete(key)
	return &pb.NickServRPCResponse{
		Body: &pb.NickServRPCResponse_Delete_{},
	}, err
}

func (s *NickServ) validUntil() uint64 {
	return uint64(time.Now().UTC().Unix()) + s.tokenTTL
}

// Less implements llrb.Item
func (p *nickServItem) Less(oi llrb.Item) bool {
	if o, ok := oi.(*nickServItem); ok {
		return bytes.Compare(p.key, o.key) == -1
	}
	return !oi.Less(p)
}

type NickServStore struct {
	// public key -> record
	records *llrb.LLRB
	// nick -> record
	nicks map[string]*nickServItem
	kv    kv.RWStore
	id    uint64
	sync.Mutex
}

func NewNickServStore(kvStore kv.RWStore, nickservID uint64) (*NickServStore, error) {
	store := &NickServStore{
		id:      nickservID,
		nicks:   make(map[string]*nickServItem),
		records: llrb.New(),
		kv:      kvStore,
	}

	records, err := dao.GetAllNickRecords(kvStore, nickservID)
	if err != nil {
		return nil, fmt.Errorf("could not retrieve existring nickserv records: %w", err)
	}

	// pupulate in memory stores from disk
	for _, record := range records {
		item := newNickServItem(record)
		store.nicks[record.Nick] = item
		store.records.ReplaceOrInsert(item)
	}

	return store, nil
}

func (s *NickServStore) Insert(r *pb.NickservNick) (*pb.NickservNick, error) {
	s.Lock()
	defer s.Unlock()

	id, err := dao.GenerateSnowflake()
	if err != nil {
		return nil, err
	}
	now := uint64(time.Now().UTC().Unix())

	record := &pb.NickservNick{
		CreatedTimestamp:         now,
		UpdatedTimestamp:         now,
		Id:                       id,
		Key:                      r.Key,
		Nick:                     r.Nick,
		RemainingNameChangeQuota: r.RemainingNameChangeQuota,
		Roles:                    r.Roles,
	}

	item := newNickServItem(record)
	_, ok := s.nicks[strings.ToLower(record.Nick)]
	if ok {
		return nil, ErrNameAlreadyTaken
	}
	_, ok = s.records.Get(item).(*nickServItem)
	if ok {
		return nil, ErrAlreadyExists
	}

	err = dao.UpsertNickRecord(s.kv, record, s.id)
	if err != nil {
		return nil, err
	}

	s.records.ReplaceOrInsert(item)
	s.nicks[strings.ToLower(record.Nick)] = item
	return proto.Clone(record).(*pb.NickservNick), nil
}

func (s *NickServStore) Delete(key []byte) error {
	s.Lock()
	defer s.Unlock()

	toBeDeleted, err := s.Retrieve(key)
	if err != nil {
		return err
	}

	err = dao.DeleteNickRecord(s.kv, s.id, toBeDeleted.Id)
	if err != nil {
		return err
	}

	keyItem := &nickServItem{key: key}
	s.records.Delete(keyItem)
	return nil
}

func (s *NickServStore) Update(newRecord *pb.NickservNick) (*pb.NickservNick, error) {
	s.Lock()
	defer s.Unlock()

	_, ok := s.nicks[strings.ToLower(newRecord.Nick)]
	if ok {
		return nil, ErrNameAlreadyTaken
	}

	prev, ok := s.records.Get(&nickServItem{key: newRecord.Key}).(*nickServItem)
	if !ok {
		return nil, ErrNotFound
	}

	record := prev.record
	oldNick := record.Nick
	record.UpdatedTimestamp = uint64(time.Now().UTC().Unix())
	record.Nick = newRecord.Nick
	record.RemainingNameChangeQuota = newRecord.RemainingNameChangeQuota
	record.Roles = newRecord.Roles

	err := dao.UpsertNickRecord(s.kv, record, s.id)
	if err != nil {
		return nil, err
	}

	// update nick index if required
	if oldNick != record.Nick {
		delete(s.nicks, strings.ToLower(oldNick))
		s.nicks[strings.ToLower(record.Nick)] = prev
	}
	return proto.Clone(record).(*pb.NickservNick), nil
}

func (s *NickServStore) Retrieve(key []byte) (*pb.NickservNick, error) {
	s.Lock()
	defer s.Unlock()
	item := s.records.Get(&nickServItem{key: key})

	if item == nil {
		return nil, ErrNotFound
	}
	old := item.(*nickServItem).record
	return proto.Clone(old).(*pb.NickservNick), nil
}

type nickServItem struct {
	key    []byte
	record *pb.NickservNick
}

func newNickServItem(r *pb.NickservNick) *nickServItem {
	return &nickServItem{
		key:    r.Key,
		record: r,
	}
}

// serializeNickToken returns a stable byte representation of a NickServToken
func serializeNickToken(token *pb.NickServToken) ([]byte, int) {
	var rolesLength int
	for _, role := range token.Roles {
		rolesLength += len(role)
	}

	b := make([]byte, len(token.Key)+len(token.Nick)+rolesLength+8)

	n := copy(b, token.Key)
	n += copy(b[n:], []byte(token.Nick))

	sort.Strings(token.Roles)
	for _, role := range token.Roles {
		n += copy(b[n:], []byte(role))
	}
	binary.BigEndian.PutUint64(b[n:], token.ValidUntil)
	n += 8

	return b, n
}

func signNickToken(token *pb.NickServToken, key *pb.Key) error {
	tokenBytes, _ := serializeNickToken(token)

	switch key.Type {
	case pb.KeyType_KEY_TYPE_ED25519:
		if len(key.Private) != ed25519.PrivateKeySize {
			return dao.ErrInvalidKeyLength
		}
		token.Signature = ed25519.Sign(key.Private, tokenBytes)
		return nil
	default:
		return dao.ErrUnsupportedKeyType
	}
}

func NewNickServClient(svc *NetworkServices, key, salt []byte) (*NickServClient, error) {
	port, err := svc.Network.ReservePort()
	if err != nil {
		return nil, err
	}

	ctx, cancel := context.WithCancel(context.Background())

	c := &NickServClient{
		close:     cancel,
		svc:       svc,
		port:      port,
		responses: make(map[uint64]chan *pb.NickServRPCResponse),
	}

	go c.syncAddr(ctx, svc, key, salt)

	return nil, nil
}

type NickServClient struct {
	close     context.CancelFunc
	closeOnce sync.Once
	svc       *NetworkServices
	responses map[uint64]chan *pb.NickServRPCResponse
	hostAddr  atomic.Value
	port      uint16
}

func (c *NickServClient) syncAddr(ctx context.Context, svc *NetworkServices, key, salt []byte) {
	var nextTick time.Duration
	for {
		select {
		case <-time.After(nextTick):
			addr, err := getHostAddr(ctx, svc, key, salt)
			if err != nil {
				nextTick = syncAddrRetryIvl
				continue
			}

			c.hostAddr.Store(addr)

			nextTick = syncAddrRefreshIvl
		case <-ctx.Done():
			return
		}
	}
}

// Close ...
func (c *NickServClient) Close() {
	c.closeOnce.Do(func() {
		c.close()
	})
}

func (c *NickServClient) MessageHandler(msg *pb.NickServRPCResponse) {
	ch, ok := c.responses[msg.RequestId]
	if !ok {
		return
	}
	select {
	case ch <- msg:
	default:
	}
}

func (c *NickServClient) registerResponseChan() (uint64, error) {
	rid, err := dao.GenerateSnowflake()
	if err != nil {
		return 0, err
	}

	c.responses[rid] = make(chan *pb.NickServRPCResponse)

	return rid, nil
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
	rid, err := c.registerResponseChan()
	if err != nil {
		return nil, err
	}

	msg.RequestId = rid

	msgBytes, err := proto.Marshal(msg)
	if err != nil {
		return nil, err
	}

	addr, ok := c.hostAddr.Load().(*hostAddr)
	if !ok {
		return nil, errors.New("unable to get host address")
	}

	err = c.svc.Network.Send(addr.HostID, addr.Port, c.port, msgBytes)
	if err != nil {
		return nil, err
	}

	return c.awaitResponse(ctx, rid)
}

func VerifyNickServToken(token *pb.NickServToken) (bool, error) {
	tokenBytes, _ := serializeNickToken(token)
	if len(token.Key) != ed25519.PublicKeySize {
		return false, dao.ErrInvalidKeyLength
	}
	return ed25519.Verify(token.Key, tokenBytes, token.Signature), nil
}
