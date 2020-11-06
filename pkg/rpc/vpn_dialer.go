package rpc

import (
	"context"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/golang/protobuf/proto"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

// VPNDialer ...
type VPNDialer struct {
	Logger *zap.Logger
	Client *vpn.Client
	Key    []byte
	Salt   []byte
}

// Dial ...
func (d *VPNDialer) Dial(ctx context.Context, dispatcher Dispatcher) (Transport, error) {
	port, err := d.Client.Network.ReservePort()
	if err != nil {
		return nil, err
	}

	c := &VPNTransport{
		ctx:        ctx,
		logger:     d.Logger,
		client:     d.Client,
		key:        d.Key,
		salt:       d.Salt,
		port:       port,
		dispatcher: dispatcher,
	}

	if err := d.Client.Network.SetHandler(port, c); err != nil {
		return nil, err
	}
	return c, nil
}

// VPNServerDialer ...
type VPNServerDialer struct {
	Logger *zap.Logger
	Client *vpn.Client
	Key    *pb.Key
	Salt   []byte
}

// Dial ...
func (d *VPNServerDialer) Dial(ctx context.Context, dispatcher Dispatcher) (Transport, error) {
	port, err := d.Client.Network.ReservePort()
	if err != nil {
		return nil, err
	}

	c := &VPNTransport{
		ctx:        ctx,
		logger:     d.Logger,
		client:     d.Client,
		key:        d.Key.Public,
		salt:       d.Salt,
		port:       port,
		dispatcher: dispatcher,
	}

	addr := &HostAddr{
		HostID: d.Client.Host.VNIC().ID(),
		Port:   port,
	}
	if err := PublishHostAddr(ctx, d.Client, d.Key, d.Salt, addr); err != nil {
		return nil, err
	}

	if err := d.Client.Network.SetHandler(port, c); err != nil {
		return nil, err
	}
	return c, nil
}

// VPNTransport ...
type VPNTransport struct {
	ctx        context.Context
	logger     *zap.Logger
	client     *vpn.Client
	key        []byte
	salt       []byte
	port       uint16
	calls      vpnCallMap
	dispatcher Dispatcher
}

// HandleMessage ...
func (t *VPNTransport) HandleMessage(msg *vpn.Message) (bool, error) {
	m := &pb.Call{}
	if err := proto.Unmarshal(msg.Body, m); err != nil {
		return false, nil
	}

	addr := &HostAddr{
		HostID: msg.Trailers[0].HostID,
		Port:   msg.Header.SrcPort,
	}

	call := NewCallIn(t.ctx, m, t.calls.Get(addr, m.ParentId))
	t.calls.Insert(addr, call)

	go func() {
		go t.dispatcher.Dispatch(call)
		call.SendResponse(func(ctx context.Context, res *pb.Call) error {
			return t.call(ctx, res, addr)
		})
		t.calls.Delete(addr, m.Id)
	}()

	return false, nil
}

func (t *VPNTransport) call(ctx context.Context, call *pb.Call, addr *HostAddr) error {
	return t.client.Network.SendProto(addr.HostID, addr.Port, t.port, call)
}

// Call ...
func (t *VPNTransport) Call(call *CallOut, fn ResponseFunc) error {
	addr, err := GetHostAddr(call.Context(), t.client, t.key, t.salt)
	if err != nil {
		return err
	}

	t.calls.Insert(addr, call)
	defer t.calls.Delete(addr, call.ID())

	err = call.SendRequest(func(ctx context.Context, res *pb.Call) error {
		return t.call(ctx, res, addr)
	})
	if err != nil {
		return err
	}

	return fn()
}

type vpnCallMap struct {
	mu sync.Mutex
	t  llrb.LLRB
}

func (m *vpnCallMap) Insert(addr *HostAddr, call Call) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.t.InsertNoReplace(&vpnCall{addr, call.ID(), call})
}

func (m *vpnCallMap) Get(addr *HostAddr, id uint64) Call {
	m.mu.Lock()
	defer m.mu.Unlock()
	if pc := m.t.Get(&vpnCall{addr: addr, id: id}); pc != nil {
		return pc.(*vpnCall).call
	}
	return nil
}

func (m *vpnCallMap) Delete(addr *HostAddr, id uint64) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.t.Delete(&vpnCall{addr: addr, id: id})
}

type vpnCall struct {
	addr *HostAddr
	id   uint64
	call Call
}

func (c *vpnCall) Less(o llrb.Item) bool {
	if o, ok := o.(*vpnCall); ok {
		if c.addr.HostID.Equals(o.addr.HostID) {
			if c.addr.Port == o.addr.Port {
				return c.id < o.id
			}
			return c.addr.Port < o.addr.Port
		}
		return c.addr.HostID.Less(o.addr.HostID)
	}
	return !o.Less(c)
}
