package dialer

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"sync"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/pool"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	rpcapi "github.com/MemeLabs/protobuf/pkg/apis/rpc"
	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

// VPNDialer ...
type VPNDialer struct {
	Logger   *zap.Logger
	Node     *vpn.Node
	Resolver HostAddrResolver
	CertFunc VPNCertFunc
}

// Dial ...
func (d *VPNDialer) Dial(ctx context.Context, dispatcher rpc.Dispatcher) (rpc.Transport, error) {
	port, err := d.Node.Network.ReservePort()
	if err != nil {
		return nil, err
	}

	return &VPNTransport{
		ctx:         ctx,
		logger:      d.Logger,
		node:        d.Node,
		resolver:    d.Resolver,
		port:        port,
		dispatcher:  dispatcher,
		certificate: d.CertFunc,
	}, nil
}

// VPNServerDialer ...
type VPNServerDialer struct {
	Logger    *zap.Logger
	Node      *vpn.Node
	Port      uint16
	Publisher HostAddrPublisher
	CertFunc  VPNCertFunc
}

// Dial ...
func (d *VPNServerDialer) Dial(ctx context.Context, dispatcher rpc.Dispatcher) (rpc.Transport, error) {
	c := &VPNTransport{
		ctx:         ctx,
		logger:      d.Logger,
		node:        d.Node,
		port:        d.Port,
		dispatcher:  dispatcher,
		certificate: d.CertFunc,
	}

	if c.port == 0 {
		port, err := d.Node.Network.ReservePort()
		if err != nil {
			return nil, err
		}
		c.port = port
	}

	if d.Publisher != nil {
		addr := &HostAddr{
			HostID: d.Node.Host.VNIC().ID(),
			Port:   c.port,
		}
		if err := d.Publisher.Publish(ctx, d.Node, addr); err != nil {
			return nil, err
		}
	}

	return c, nil
}

// VPNTransport ...
type VPNTransport struct {
	ctx         context.Context
	logger      *zap.Logger
	node        *vpn.Node
	resolver    HostAddrResolver
	certificate VPNCertFunc
	port        uint16
	callsIn     vpnCallMap
	callsOut    vpnCallMap
	dispatcher  rpc.Dispatcher
}

// Listen ...
func (t *VPNTransport) Listen() error {
	if err := t.node.Network.SetHandler(t.port, t); err != nil {
		return err
	}

	<-t.ctx.Done()

	t.node.Network.RemoveHandler(t.port)
	t.node.Network.ReleasePort(t.port)

	return t.ctx.Err()
}

// HandleMessage ...
func (t *VPNTransport) HandleMessage(msg *vpn.Message) error {
	req := &rpcapi.Call{}
	if err := proto.Unmarshal(msg.Body, req); err != nil {
		return fmt.Errorf("unmarshaling rpc: %w", err)
	}

	cert := &certificate.Certificate{}
	if err := proto.Unmarshal(req.Headers[vpnCertificateHeader], cert); err != nil {
		return fmt.Errorf("unmarshaling rpc: %w", err)
	}
	if err := t.verifyMessage(msg, req, cert); err != nil {
		return fmt.Errorf("verifying rpc: %w", err)
	}

	addr := &HostAddr{
		HostID: msg.SrcHostID(),
		Port:   msg.Header.SrcPort,
	}

	ctx := context.WithValue(t.ctx, vpnCertificateKey, cert)
	parentCallAccessor := &vpnParentCallAccessor{
		addr:     addr,
		id:       req.ParentId,
		callsIn:  &t.callsIn,
		callsOut: &t.callsOut,
	}
	send := func(ctx context.Context, res *rpcapi.Call) error {
		return t.send(ctx, res, addr)
	}
	call := rpc.NewCallIn(ctx, req, parentCallAccessor, send)

	t.callsIn.Insert(addr, call)
	t.dispatcher.Dispatch(call, func() { t.callsIn.Delete(addr, req.Id) })

	return nil
}

func (t *VPNTransport) send(ctx context.Context, call *rpcapi.Call, addr *HostAddr) error {
	cert := t.certificate()

	opt := proto.MarshalOptions{}
	p := pool.Get(opt.Size(cert))
	defer pool.Put(p)
	b, err := opt.MarshalAppend((*p)[:0], cert)
	if err != nil {
		return err
	}
	call.Headers[vpnCertificateHeader] = b

	return t.node.Network.SendProto(addr.HostID, addr.Port, t.port, call)
}

// Call ...
func (t *VPNTransport) Call(call *rpc.CallOut, fn rpc.ResponseFunc) error {
	addr, err := t.resolver.Resolve(call.Context(), t.node)
	if err != nil {
		return err
	}

	t.callsOut.Insert(addr, call)
	defer t.callsOut.Delete(addr, call.ID())

	err = call.SendRequest(func(ctx context.Context, res *rpcapi.Call) error {
		return t.send(ctx, res, addr)
	})
	if err != nil {
		return err
	}

	return fn()
}

const vpnCertificateHeader = "certificate"

func (t *VPNTransport) verifyMessage(msg *vpn.Message, req *rpcapi.Call, cert *certificate.Certificate) error {
	if !bytes.Equal(cert.GetKey(), msg.Trailer.Entries[0].HostID.Bytes(nil)) {
		return errors.New("certificate host id mismatch")
	}

	if !bytes.Equal(dao.CertificateRoot(t.certificate()).Key, cert.GetParent().GetParent().GetKey()) {
		return errors.New("network key mismatch")
	}
	if err := dao.VerifyCertificate(cert); err != nil {
		return err
	}
	return nil
}

// VPNCertFunc ...
type VPNCertFunc func() *certificate.Certificate

type vpnCertificateKeyType struct{}

var vpnCertificateKey vpnCertificateKeyType

// VPNCertificate ...
func VPNCertificate(ctx context.Context) *certificate.Certificate {
	return ctx.Value(vpnCertificateKey).(*certificate.Certificate)
}

type vpnCallMap struct {
	mu sync.Mutex
	t  llrb.LLRB
}

func (m *vpnCallMap) Insert(addr *HostAddr, call rpc.Call) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.t.InsertNoReplace(&vpnCall{addr, call.ID(), call})
}

func (m *vpnCallMap) Get(addr *HostAddr, id uint64) rpc.Call {
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
	call rpc.Call
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

func (a *vpnParentCallAccessor) ParentCallIn() *rpc.CallIn {
	if p := a.callsOut.Get(a.addr, a.id); p != nil {
		return p.(*rpc.CallIn)
	}
	return nil
}

func (a *vpnParentCallAccessor) ParentCallOut() *rpc.CallOut {
	if p := a.callsOut.Get(a.addr, a.id); p != nil {
		return p.(*rpc.CallOut)
	}
	return nil
}
