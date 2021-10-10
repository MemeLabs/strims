package dialer

import (
	"bytes"
	"errors"
	"sync"

	control "github.com/MemeLabs/go-ppspp/internal"
	"github.com/MemeLabs/go-ppspp/internal/dao"
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

var _ control.DialerControl = &Control{}

// NewControl ...
func NewControl(logger *zap.Logger, vpn *vpn.Host, profile *profilev1.Profile) *Control {
	return &Control{
		logger:  logger,
		vpn:     vpn,
		profile: profile,
	}
}

// Control ...
type Control struct {
	logger  *zap.Logger
	vpn     *vpn.Host
	profile *profilev1.Profile
	lock    sync.Mutex
	certs   llrb.LLRB
}

// ReplaceOrInsertNetwork ...
func (t *Control) ReplaceOrInsertNetwork(network *networkv1.Network) {
	csr, err := dao.NewCertificateRequest(t.vpn.VNIC().Key(), certificate.KeyUsage_KEY_USAGE_SIGN)
	if err != nil {
		return
	}

	now := timeutil.Now()
	notAfter := timeutil.Unix(int64(network.Certificate.NotAfter), 0)
	if now.After(notAfter) {
		return
	}

	cert, err := dao.SignCertificateRequest(csr, notAfter.Sub(now), t.profile.Key)
	if err != nil {
		return
	}
	cert.ParentOneof = &certificate.Certificate_Parent{Parent: network.Certificate}

	t.lock.Lock()
	defer t.lock.Unlock()

	it := t.certs.Get(&hostCertKey{dao.NetworkKey(network)})
	if it == nil {
		t.certs.ReplaceOrInsert(&hostCert{cert: cert})
	} else {
		it.(*hostCert).Store(cert)
	}
}

// RemoveNetwork ...
func (t *Control) RemoveNetwork(network *networkv1.Network) {
	t.lock.Lock()
	defer t.lock.Unlock()

	t.certs.Delete(&hostCertKey{dao.NetworkKey(network)})
}

func (t *Control) hostCertAndVPNNode(networkKey []byte) (*hostCert, *vpn.Node, error) {
	t.lock.Lock()
	defer t.lock.Unlock()

	cert := t.certs.Get(&hostCertKey{networkKey})
	if cert == nil {
		return nil, nil, errors.New("host certificate not found")
	}
	node, ok := t.vpn.Node(networkKey)
	if !ok {
		return nil, nil, errors.New("network not found")
	}
	return cert.(*hostCert), node, nil
}

// ServerDialer ...
func (t *Control) ServerDialer(networkKey []byte, key *key.Key, salt []byte) (rpc.Dialer, error) {
	cert, node, err := t.hostCertAndVPNNode(networkKey)
	if err != nil {
		return nil, err
	}

	return &VPNServerDialer{
		Logger:   t.logger,
		Node:     node,
		Key:      key,
		Salt:     salt,
		CertFunc: cert.Load,
	}, nil
}

// Server ...
func (t *Control) Server(networkKey []byte, key *key.Key, salt []byte) (*rpc.Server, error) {
	dialer, err := t.ServerDialer(networkKey, key, salt)
	if err != nil {
		return nil, err
	}
	return rpc.NewServer(t.logger, dialer), nil
}

// ClientDialer ...
func (t *Control) ClientDialer(networkKey, key, salt []byte) (rpc.Dialer, error) {
	cert, node, err := t.hostCertAndVPNNode(networkKey)
	if err != nil {
		return nil, err
	}

	return &VPNDialer{
		Logger:   t.logger,
		Node:     node,
		Key:      key,
		Salt:     salt,
		CertFunc: cert.Load,
	}, nil
}

// Client ...
func (t *Control) Client(networkKey, key, salt []byte) (*rpc.Client, error) {
	dialer, err := t.ClientDialer(networkKey, key, salt)
	if err != nil {
		return nil, err
	}
	return rpc.NewClient(t.logger, dialer)
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
	lock sync.Mutex
	cert *certificate.Certificate
}

func (h *hostCert) Store(cert *certificate.Certificate) {
	h.lock.Lock()
	defer h.lock.Unlock()
	h.cert = cert
}

func (h *hostCert) Load() *certificate.Certificate {
	h.lock.Lock()
	defer h.lock.Unlock()
	return h.cert
}

func (h *hostCert) Key() []byte {
	h.lock.Lock()
	defer h.lock.Unlock()
	return dao.CertificateRoot(h.cert).Key
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
