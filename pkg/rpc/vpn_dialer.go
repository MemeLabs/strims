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

	return &VPNTransport{
		ctx:        ctx,
		logger:     d.Logger,
		client:     d.Client,
		key:        d.Key,
		salt:       d.Salt,
		port:       port,
		dispatcher: dispatcher,
	}, nil
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
	callsIn    vpnCallMap
	callsOut   vpnCallMap
	dispatcher Dispatcher
}

// Listen ...
func (t *VPNTransport) Listen() error {
	if err := t.client.Network.SetHandler(t.port, t); err != nil {
		return err
	}

	<-t.ctx.Done()

	t.client.Network.RemoveHandler(t.port)
	t.client.Network.ReleasePort(t.port)

	return t.ctx.Err()
}

// HandleMessage ...
func (t *VPNTransport) HandleMessage(msg *vpn.Message) (bool, error) {
	req := &pb.Call{}
	if err := proto.Unmarshal(msg.Body, req); err != nil {
		return false, nil
	}

	addr := &HostAddr{
		HostID: msg.SrcHostID(),
		Port:   msg.Header.SrcPort,
	}

	parentCallAccessor := &vpnParentCallAccessor{
		addr:     addr,
		id:       req.ParentId,
		callsIn:  &t.callsIn,
		callsOut: &t.callsOut,
	}
	call := NewCallIn(t.ctx, req, parentCallAccessor)
	t.callsIn.Insert(addr, call)

	go t.dispatcher.Dispatch(call)
	go func() {
		call.SendResponse(func(ctx context.Context, res *pb.Call) error {
			return t.call(ctx, res, addr)
		})
		t.callsIn.Delete(addr, req.Id)
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

	t.callsOut.Insert(addr, call)
	defer t.callsOut.Delete(addr, call.ID())

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

type vpnParentCallAccessor struct {
	addr     *HostAddr
	id       uint64
	callsIn  *vpnCallMap
	callsOut *vpnCallMap
}

func (a *vpnParentCallAccessor) ParentCallIn() *CallIn {
	if p := a.callsOut.Get(a.addr, a.id); p != nil {
		return p.(*CallIn)
	}
	return nil
}

func (a *vpnParentCallAccessor) ParentCallOut() *CallOut {
	if p := a.callsOut.Get(a.addr, a.id); p != nil {
		return p.(*CallOut)
	}
	return nil
}
