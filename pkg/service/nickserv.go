package service

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"io"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/prefixstream"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

var defaultNameChangeQuota uint32 = 5

// TODO: crud handlers
// TODO: dao helpers for storing nickservtokens
// TODO: sign/verify nickservtoken

func NewNickServ(svc *NetworkServices, cfg *pb.ServerConfig, salt []byte) (*NickServ, error) {
	w, err := ppspp.NewWriter(ppspp.WriterOptions{
		// SwarmOptions: ppspp.NewDefaultSwarmOptions(),
		SwarmOptions: ppspp.SwarmOptions{
			LiveWindow: 1 << 10, // 1MB
			ChunkSize:  128,
		},
		Key: cfg.Key,
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
	newSwarmPeerManager(ctx, svc, getPeersGetter(ctx, svc, cfg.Key.Public, salt))

	b, err := proto.Marshal(&pb.NetworkAddress{
		HostId: svc.Host.ID().Bytes(nil),
		Port:   uint32(port),
	})
	if err != nil {
		cancel()
		return nil, err
	}
	_, err = svc.HashTable.Set(ctx, cfg.Key, salt, b)
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
	var m pb.NickServEvent
	if err := proto.Unmarshal(msg.Body, &m); err != nil {
		return true, err
	}

	// TODO: verify `source_public_key`

	switch b := m.Body.(type) {
	case *pb.NickServEvent_Create_:
		err = s.handleCreate(m.RequestId, m.SourcePublicKey, b.Create)
	case *pb.NickServEvent_Retrieve_:
		err = s.handleRetrieve(b.Retrieve)
	case *pb.NickServEvent_Update_:
		err = s.handleUpdate(b.Update)
	case *pb.NickServEvent_Delete_:
		err = s.handleDelete(b.Delete)
	default:
		err = errors.New("unexpected message type")
	}

	return true, nil
}

func (s *NickServ) handleCreate(id uint64, key *pb.Key, msg *pb.NickServEvent_Create) error {
	now := time.Now()
	r := &pb.NickRecord{
		PeerPublicKey:            key,
		Name:                     msg.Nick,
		CreatedTimestamp:         uint64(now.Unix()),
		UpdatedTimestamp:         uint64(now.Unix()),
		RemainingNameChangeQuota: defaultNameChangeQuota,
		Roles:                    []string{},
	}
	return s.store.Insert(r)
}

func (s *NickServ) handleRetrieve(msg *pb.NickServEvent_Retrieve) error {
	return fmt.Errorf("unimplemented")
}

func (s *NickServ) handleUpdate(msg *pb.NickServEvent_Update) error {
	return fmt.Errorf("unimplemented")
}

func (s *NickServ) handleDelete(msg *pb.NickServEvent_Delete) error {
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
	ctx       context.Context
	close     context.CancelFunc
	closeOnce sync.Once
	svc       *NetworkServices
	requests  map[uint64]chan struct{}
	swarm     *ppspp.Swarm
	addrReady chan struct{}
	addr      atomic.Value
	port      uint16
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

	b, err := proto.Marshal(&pb.NickServEvent{
		RequestId: id,
	})
	if err != nil {
		return err
	}

	addr := c.addr.Load().(*hostAddr)
	return c.svc.Network.Send(addr.HostID, addr.Port, c.port, b)
}

func readNickServEvents(swarm *ppspp.Swarm, messages chan *pb.NickServEvent) {
	r := prefixstream.NewReader(swarm.Reader())
	b := bytes.NewBuffer(nil)
	for {
		b.Reset()
		if _, err := io.Copy(b, r); err != nil {
			return
		}

		var msg pb.NickServEvent
		if err := proto.Unmarshal(b.Bytes(), &msg); err != nil {
			continue
		}
	}
}

type NickServStore struct {
	logger  *zap.Logger
	lock    sync.Mutex
	records *llrb.LLRB
}

func (s *NickServStore) Insert(r *pb.NickRecord) error {
	return fmt.Errorf("unimplemented")
}

func (s *NickServStore) Delete(r *pb.NickRecord) error {
	return fmt.Errorf("unimplemented")
}

func (s *NickServStore) Update(r *pb.NickRecord) error {
	return fmt.Errorf("unimplemented")
}

func (s *NickServStore) Retrieve(nick string) error {
	return fmt.Errorf("unimplemented")
}

var nextSnowflakeID uint64

// generate a 53 bit locally unique id
func generateSnowflake() (uint64, error) {
	seconds := uint64(time.Since(time.Date(2020, 0, 0, 0, 0, 0, 0, time.UTC)) / time.Second)
	sequence := atomic.AddUint64(&nextSnowflakeID, 1) << 32
	return (seconds | sequence) & 0x1fffffffffffff, nil
}
