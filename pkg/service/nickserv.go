package service

import (
	"context"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/prefixstream"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"google.golang.org/protobuf/proto"
)

// TODO: message routing to crud handlers
// TODO: crud handlers
// TODO: dao helpers for storing nickservtokens

func NewNickServer(svc *NetworkServices, cfg *pb.ServerConfig, salt []byte) (*NickServer, error) {
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

	s := &NickServer{
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

type NickServer struct {
	//	cfg             *pb.ServerConfig
	close           context.CancelFunc
	swarm           *ppspp.Swarm
	closeOnce       sync.Once
	svc             *NetworkServices
	w               *prefixstream.Writer
	roles           []string
	tokenttl        uint64
	nameChangeQuota uint32 // smaller?
}

func (s *NickServer) Close() {
	s.closeOnce.Do(func() {
		s.close()
		s.svc.Swarms.CloseSwarm(s.swarm.ID())
	})
}

func (s *NickServer) HandleMessage(msg *vpn.Message) (forward bool, err error) {
	return true, nil
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
