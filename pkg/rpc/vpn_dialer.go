package rpc

import (
	"bytes"
	"context"
	"errors"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/golang/protobuf/proto"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

// VPNDialer ...
type VPNDialer struct {
	Logger   *zap.Logger
	Node     *vpn.Node
	Key      []byte
	Salt     []byte
	CertFunc VPNCertFunc
}

// Dial ...
func (d *VPNDialer) Dial(ctx context.Context, dispatcher Dispatcher) (Transport, error) {
	port, err := d.Node.Network.ReservePort()
	if err != nil {
		return nil, err
	}

	return &VPNTransport{
		ctx:         ctx,
		logger:      d.Logger,
		node:        d.Node,
		key:         d.Key,
		salt:        d.Salt,
		port:        port,
		dispatcher:  dispatcher,
		certificate: d.CertFunc,
	}, nil
}

// VPNServerDialer ...
type VPNServerDialer struct {
	Logger   *zap.Logger
	Node     *vpn.Node
	Key      *pb.Key
	Salt     []byte
	CertFunc VPNCertFunc
}

// Dial ...
func (d *VPNServerDialer) Dial(ctx context.Context, dispatcher Dispatcher) (Transport, error) {
	port, err := d.Node.Network.ReservePort()
	if err != nil {
		return nil, err
	}

	c := &VPNTransport{
		ctx:         ctx,
		logger:      d.Logger,
		node:        d.Node,
		key:         d.Key.Public,
		salt:        d.Salt,
		port:        port,
		dispatcher:  dispatcher,
		certificate: d.CertFunc,
	}

	addr := &HostAddr{
		HostID: d.Node.Host.VNIC().ID(),
		Port:   port,
	}
	if err := PublishHostAddr(ctx, d.Node, d.Key, d.Salt, addr); err != nil {
		return nil, err
	}

	return c, nil
}

// VPNTransport ...
type VPNTransport struct {
	ctx         context.Context
	logger      *zap.Logger
	node        *vpn.Node
	key         []byte
	salt        []byte
	certificate VPNCertFunc
	port        uint16
	callsIn     vpnCallMap
	callsOut    vpnCallMap
	dispatcher  Dispatcher
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
	req := &pb.Call{}
	if err := proto.Unmarshal(msg.Body, req); err != nil {
		return err
	}

	cert := &pb.Certificate{}
	if err := proto.Unmarshal(req.Headers[vpnCertificateHeader], cert); err != nil {
		return err
	}
	if err := t.verifyMessage(msg, req, cert); err != nil {
		return err
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
	send := func(ctx context.Context, res *pb.Call) error {
		return t.send(ctx, res, addr)
	}
	call := NewCallIn(ctx, req, parentCallAccessor, send)

	t.callsIn.Insert(addr, call)
	go func() {
		t.dispatcher.Dispatch(call)
		t.callsIn.Delete(addr, req.Id)
	}()

	return nil
}

func (t *VPNTransport) send(ctx context.Context, call *pb.Call, addr *HostAddr) error {
	b := callBuffers.Get().(*proto.Buffer)
	defer callBuffers.Put(b)

	b.Reset()
	if err := b.Marshal(t.certificate()); err != nil {
		return err
	}
	call.Headers[vpnCertificateHeader] = b.Bytes()

	return t.node.Network.SendProto(addr.HostID, addr.Port, t.port, call)
}

// Call ...
func (t *VPNTransport) Call(call *CallOut, fn ResponseFunc) error {
	addr, err := GetHostAddr(call.Context(), t.node, t.key, t.salt)
	if err != nil {
		return err
	}

	t.callsOut.Insert(addr, call)
	defer t.callsOut.Delete(addr, call.ID())

	err = call.SendRequest(func(ctx context.Context, res *pb.Call) error {
		return t.send(ctx, res, addr)
	})
	if err != nil {
		return err
	}

	return fn()
}

const vpnCertificateHeader = "certificate"

func (t *VPNTransport) verifyMessage(msg *vpn.Message, req *pb.Call, cert *pb.Certificate) error {
	if !bytes.Equal(cert.GetKey(), msg.Trailer.Entries[0].HostID.Bytes(nil)) {
		return errors.New("certificate host id mismatch")
	}

	if !bytes.Equal(dao.GetRootCert(t.certificate()).Key, cert.GetParent().GetParent().GetKey()) {
		return errors.New("network key mismatch")
	}
	if err := dao.VerifyCertificate(cert); err != nil {
		return err
	}
	if !msg.Verify(0) {
		return errors.New("invalid message signature")
	}
	return nil
}

// VPNCertFunc ...
type VPNCertFunc func() *pb.Certificate

type vpnCertificateKeyType struct{}

var vpnCertificateKey vpnCertificateKeyType

// VPNCertificate ...
func VPNCertificate(ctx context.Context) *pb.Certificate {
	return ctx.Value(vpnCertificateKey).(*pb.Certificate)
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
