package dialer

import (
	"bytes"
	"errors"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/rpc"
	"github.com/MemeLabs/go-ppspp/pkg/vpn"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

// NewControl ...
func NewControl(logger *zap.Logger, vpn *vpn.Host, profile *pb.Profile) *Control {
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
	profile *pb.Profile
	lock    sync.Mutex
	certs   llrb.LLRB
}

// ReplaceOrInsertNetwork ...
func (t *Control) ReplaceOrInsertNetwork(network *pb.Network) {
	csr, err := dao.NewCertificateRequest(t.vpn.VNIC().Key(), pb.KeyUsage_KEY_USAGE_SIGN)
	if err != nil {
		return
	}

	now := time.Now()
	notAfter := time.Unix(int64(network.Certificate.NotAfter), 0)
	if now.After(notAfter) {
		return
	}

	cert, err := dao.SignCertificateRequest(csr, notAfter.Sub(now), t.profile.Key)
	if err != nil {
		return
	}
	cert.ParentOneof = &pb.Certificate_Parent{Parent: network.Certificate}

	t.lock.Lock()
	defer t.lock.Unlock()

	it := t.certs.Get(&hostCertKey{dao.GetRootCert(network.Certificate).Key})
	if it == nil {
		t.certs.ReplaceOrInsert(&hostCert{cert: cert})
	} else {
		it.(*hostCert).Store(cert)
	}
}

// RemoveNetwork ...
func (t *Control) RemoveNetwork(network *pb.Network) {
	t.lock.Lock()
	defer t.lock.Unlock()

	t.certs.Delete(&hostCertKey{dao.GetRootCert(network.Certificate).Key})
}

func (t *Control) hostCertAndVPNClient(networkKey []byte) (*hostCert, *vpn.Client, error) {
	t.lock.Lock()
	defer t.lock.Unlock()

	cert := t.certs.Get(&hostCertKey{networkKey})
	if cert == nil {
		return nil, nil, errors.New("host certificate not found")
	}
	client, ok := t.vpn.Client(networkKey)
	if !ok {
		return nil, nil, errors.New("network not found")
	}
	return cert.(*hostCert), client, nil
}

// ServerDialer ...
func (t *Control) ServerDialer(networkKey []byte, key *pb.Key, salt []byte) (rpc.Dialer, error) {
	cert, client, err := t.hostCertAndVPNClient(networkKey)
	if err != nil {
		return nil, err
	}

	return &rpc.VPNServerDialer{
		Logger:   t.logger,
		Client:   client,
		Key:      key,
		Salt:     salt,
		CertFunc: cert.Load,
	}, nil
}

// Server ...
func (t *Control) Server(networkKey []byte, key *pb.Key, salt []byte) (*rpc.Server, error) {
	dialer, err := t.ServerDialer(networkKey, key, salt)
	if err != nil {
		return nil, err
	}
	return rpc.NewServer(t.logger, dialer), nil
}

// ClientDialer ...
func (t *Control) ClientDialer(networkKey, key, salt []byte) (rpc.Dialer, error) {
	cert, client, err := t.hostCertAndVPNClient(networkKey)
	if err != nil {
		return nil, err
	}

	return &rpc.VPNDialer{
		Logger:   t.logger,
		Client:   client,
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
	cert *pb.Certificate
}

func (h *hostCert) Store(cert *pb.Certificate) {
	h.lock.Lock()
	defer h.lock.Unlock()
	h.cert = cert
}

func (h *hostCert) Load() *pb.Certificate {
	h.lock.Lock()
	defer h.lock.Unlock()
	return h.cert
}

func (h *hostCert) Key() []byte {
	h.lock.Lock()
	defer h.lock.Unlock()
	return dao.GetRootCert(h.cert).Key
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
