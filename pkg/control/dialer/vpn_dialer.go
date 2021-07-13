package dialer

import (
	"bytes"
	"context"
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"errors"
	"fmt"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/ed25519util"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/pool"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	rpcapi "github.com/MemeLabs/protobuf/pkg/apis/rpc"
	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
	"golang.org/x/crypto/curve25519"
	"google.golang.org/protobuf/proto"
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
func (d *VPNDialer) Dial(ctx context.Context, dispatcher rpc.Dispatcher) (rpc.Transport, error) {
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
	Key      *key.Key
	Salt     []byte
	CertFunc VPNCertFunc
}

// Dial ...
func (d *VPNServerDialer) Dial(ctx context.Context, dispatcher rpc.Dispatcher) (rpc.Transport, error) {
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
	c, err := t.cipher(msg.Trailer.Entries[0].HostID)
	if err != nil {
		return err
	}
	p := pool.Get(len(msg.Body))
	defer pool.Put(p)
	b, err := c.Open((*p)[:0], msg.Body)
	if err != nil {
		return err
	}

	req := &rpcapi.Call{}
	if err := proto.Unmarshal(b, req); err != nil {
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
	opt := proto.MarshalOptions{}

	cert := t.certificate()
	p0 := pool.Get(opt.Size(cert))
	defer pool.Put(p0)
	b0, err := opt.MarshalAppend((*p0)[:0], cert)
	if err != nil {
		return err
	}
	call.Headers[vpnCertificateHeader] = b0

	p1 := pool.Get(opt.Size(call))
	defer pool.Put(p1)
	b1, err := opt.MarshalAppend((*p1)[:0], call)
	if err != nil {
		return err
	}

	c, err := t.cipher(addr.HostID)
	if err != nil {
		return err
	}
	p2 := pool.Get(len(b1) + c.Overhead())
	defer pool.Put(p2)
	b2, err := c.Seal((*p2)[:0], b1)
	if err != nil {
		return err
	}

	return t.node.Network.Send(addr.HostID, addr.Port, t.port, b2)
}

// Call ...
func (t *VPNTransport) Call(call *rpc.CallOut, fn rpc.ResponseFunc) error {
	addr, err := GetHostAddr(call.Context(), t.node, t.key, t.salt)
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

func (t *VPNTransport) cipher(hostID kademlia.ID) (*transportCipher, error) {
	var ed25519Private [64]byte
	var ed25519Public [32]byte
	hostID.Bytes(ed25519Public[:])
	copy(ed25519Private[:], t.node.Host.VNIC().Key().Private)

	var curve25519Private, curve25519Public [32]byte
	ed25519util.PrivateKeyToCurve25519(&curve25519Private, &ed25519Private)
	ed25519util.PublicKeyToCurve25519(&curve25519Public, &ed25519Public)

	secret, err := curve25519.X25519(curve25519Private[:], curve25519Public[:])
	if err != nil {
		return nil, err
	}
	return newTransportCipher(secret)
}

const vpnCertificateHeader = "certificate"

func (t *VPNTransport) verifyMessage(msg *vpn.Message, req *rpcapi.Call, cert *certificate.Certificate) error {
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

func newTransportCipher(k []byte) (*transportCipher, error) {
	block, err := aes.NewCipher(k)
	if err != nil {
		return nil, err
	}
	aesgcm, err := cipher.NewGCM(block)
	if err != nil {
		return nil, err
	}

	return &transportCipher{
		cipher: aesgcm,
	}, nil
}

type transportCipher struct {
	cipher cipher.AEAD
}

func (t *transportCipher) Overhead() int {
	return t.cipher.NonceSize() + t.cipher.Overhead()
}

func (t *transportCipher) Seal(b, p []byte) ([]byte, error) {
	n := t.cipher.NonceSize()
	if _, err := rand.Read(b[:n]); err != nil {
		return nil, err
	}

	return t.cipher.Seal(b[:n], b[:n], p, nil), nil
}

func (t *transportCipher) Open(b, p []byte) ([]byte, error) {
	n := t.cipher.NonceSize()
	return t.cipher.Open(b, p[:n], p[n:], nil)
}
