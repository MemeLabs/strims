package network

import (
	"bytes"
	"context"
	"errors"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/event"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/rpcutil"
	"github.com/MemeLabs/go-ppspp/pkg/syncutil"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

var _ Dialer = &dialer{}

type Dialer interface {
	ServerDialer(ctx context.Context, networkKey []byte, port uint16, publisher HostAddrPublisher) (rpc.Dialer, error)
	Server(ctx context.Context, networkKey []byte, key *key.Key, salt []byte) (*rpc.Server, error)
	ServerWithHostAddr(ctx context.Context, networkKey []byte, port uint16) (*rpc.Server, error)
	ClientDialer(ctx context.Context, networkKey []byte, resolver HostAddrResolver) (rpc.Dialer, error)
	Client(ctx context.Context, networkKey, key, salt []byte) (*RPCClient, error)
	ClientWithHostAddr(ctx context.Context, networkKey []byte, hostID kademlia.ID, port uint16) (*RPCClient, error)
}

// newDialer ...
func newDialer(logger *zap.Logger, vpn *vpn.Host, key *key.Key) *dialer {
	return &dialer{
		logger: logger,
		vpn:    vpn,
		key:    key,
	}
}

// dialer ...
type dialer struct {
	logger    *zap.Logger
	vpn       *vpn.Host
	key       *key.Key
	lock      sync.Mutex
	certs     llrb.LLRB
	certAdded event.Observer
}

// replaceOrInsertNetwork ...
func (t *dialer) replaceOrInsertNetwork(network *networkv1.Network) {
	csr, err := dao.NewCertificateRequest(t.vpn.VNIC().Key(), certificate.KeyUsage_KEY_USAGE_SIGN)
	if err != nil {
		return
	}

	now := timeutil.Now()
	notAfter := timeutil.Unix(int64(network.Certificate.NotAfter), 0)
	if now.After(notAfter) {
		return
	}

	cert, err := dao.SignCertificateRequest(csr, notAfter.Sub(now), t.key)
	if err != nil {
		return
	}
	cert.ParentOneof = &certificate.Certificate_Parent{Parent: network.Certificate}

	t.lock.Lock()
	it := t.certs.Get(&hostCertKey{dao.NetworkKey(network)})
	if it == nil {
		t.certs.ReplaceOrInsert(&hostCert{syncutil.NewPointer(cert)})
	} else {
		it.(*hostCert).Swap(cert)
	}
	t.lock.Unlock()

	if it == nil {
		t.certAdded.Emit(struct{}{})
	}
}

// removeNetwork ...
func (t *dialer) removeNetwork(network *networkv1.Network) {
	t.lock.Lock()
	t.certs.Delete(&hostCertKey{dao.NetworkKey(network)})
	t.lock.Unlock()
}

func (t *dialer) hostCertAndVPNNode(ctx context.Context, networkKey []byte) (*hostCert, *vpn.Node, error) {
	ch := make(chan struct{}, 1)
	t.certAdded.Notify(ch)
	defer t.certAdded.StopNotifying(ch)

	for {
		t.lock.Lock()
		cert := t.certs.Get(&hostCertKey{networkKey})
		t.lock.Unlock()

		if cert == nil {
			select {
			case <-ctx.Done():
				return nil, nil, ctx.Err()
			case <-ch:
				continue
			}
		}

		node, ok := t.vpn.Node(networkKey)
		if !ok {
			return nil, nil, errors.New("network not found")
		}
		return cert.(*hostCert), node, nil
	}
}

// ServerDialer ...
func (t *dialer) ServerDialer(ctx context.Context, networkKey []byte, port uint16, publisher HostAddrPublisher) (rpc.Dialer, error) {
	cert, node, err := t.hostCertAndVPNNode(ctx, networkKey)
	if err != nil {
		return nil, err
	}

	return &VPNServerDialer{
		Logger:    t.logger,
		Node:      node,
		Port:      port,
		Publisher: publisher,
		CertFunc:  cert.Get,
	}, nil
}

// Server ...
func (t *dialer) Server(ctx context.Context, networkKey []byte, key *key.Key, salt []byte) (*rpc.Server, error) {
	dialer, err := t.ServerDialer(ctx, networkKey, 0, &DHTHostAddrPublisher{key, salt})
	if err != nil {
		return nil, err
	}
	return rpc.NewServer(t.logger, dialer), nil
}

// ServerWithHostAddr ...
func (t *dialer) ServerWithHostAddr(ctx context.Context, networkKey []byte, port uint16) (*rpc.Server, error) {
	dialer, err := t.ServerDialer(ctx, networkKey, port, nil)
	if err != nil {
		return nil, err
	}
	return rpc.NewServer(t.logger, dialer), nil
}

// ClientDialer ...
func (t *dialer) ClientDialer(ctx context.Context, networkKey []byte, resolver HostAddrResolver) (rpc.Dialer, error) {
	cert, node, err := t.hostCertAndVPNNode(ctx, networkKey)
	if err != nil {
		return nil, err
	}

	return &VPNDialer{
		Logger:   t.logger,
		Node:     node,
		Resolver: resolver,
		CertFunc: cert.Get,
	}, nil
}

// Client ...
func (t *dialer) Client(ctx context.Context, networkKey, key, salt []byte) (*RPCClient, error) {
	dialer, err := t.ClientDialer(ctx, networkKey, &DHTHostAddrResolver{key, salt})
	if err != nil {
		return nil, err
	}
	return NewRPCClient(t.logger, dialer)
}

// ClientWithHostAddr ...
func (t *dialer) ClientWithHostAddr(ctx context.Context, networkKey []byte, hostID kademlia.ID, port uint16) (*RPCClient, error) {
	dialer, err := t.ClientDialer(ctx, networkKey, &StaticHostAddrResolver{HostAddr{hostID, port}})
	if err != nil {
		return nil, err
	}
	return NewRPCClient(t.logger, dialer)
}

type hostCertKey struct {
	key []byte
}

func (h *hostCertKey) Key() []byte {
	return h.key
}

func (h *hostCertKey) Less(o llrb.Item) bool {
	return keyerLess(h, o)
}

type hostCert struct {
	syncutil.Pointer[certificate.Certificate]
}

func (h *hostCert) Key() []byte {
	return dao.CertificateRoot(h.Get()).Key
}

func (h *hostCert) Less(o llrb.Item) bool {
	return keyerLess(h, o)
}

type keyer interface {
	llrb.Item
	Key() []byte
}

func keyerLess(h keyer, o llrb.Item) bool {
	if o, ok := o.(keyer); ok {
		return bytes.Compare(h.Key(), o.Key()) == -1
	}
	return !o.Less(h)
}

const (
	RPCClientRetries = 3
	RPCClientBackoff = 2
	RPCClientDelay   = time.Second
	RPCClientTimeout = 10 * time.Second
)

func NewRPCClient(logger *zap.Logger, dialer rpc.Dialer) (*RPCClient, error) {
	c, err := rpc.NewClient(logger, dialer)
	if err != nil {
		return nil, err
	}

	rc := rpc.Caller(rpcutil.NewClientRetrier(c, RPCClientRetries, RPCClientBackoff, RPCClientDelay, RPCClientTimeout))
	if l := logger.Check(zap.DebugLevel, "enabling client logging"); l != nil {
		l.Write()
		rc = rpcutil.NewClientLogger(rc, logger)
	}

	return &RPCClient{rc, c.Close}, nil
}

type RPCClient struct {
	rpc.Caller
	close func()
}

func (c *RPCClient) Close() {
	c.close()
}
