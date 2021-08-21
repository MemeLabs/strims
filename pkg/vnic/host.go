package vnic

import (
	"errors"
	"fmt"
	"io"
	"log"
	"reflect"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/vnic/qos"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
	"go.uber.org/zap"
)

const reservedPortCount uint16 = 1000
const hostCertValidDuration = time.Hour

// default network service ports
const (
	HashTablePort uint16 = iota + 10
	PeerIndexPort

	PeerExchangePort uint16 = 1001
	TransferPort     uint16 = 1002
)

// peer link ports
const (
	NetworkInitPort uint16 = iota
	NetworkBrokerPort
	BootstrapPort
	SwarmPort
	PeerRPCClientPort
	PeerRPCServerPort
)

var (
	linksActive = promauto.NewGauge(prometheus.GaugeOpts{
		Name: "strims_vnic_links_active",
		Help: "The number of active network links",
	})
	dialCount = promauto.NewCounterVec(prometheus.CounterOpts{
		Name: "strims_vnic_dial_count",
		Help: "The total number of dialed network connections",
	}, []string{"scheme"})
	dialErrorCount = promauto.NewCounterVec(prometheus.CounterOpts{
		Name: "strims_vnic_dial_error_count",
		Help: "The total number of network connection dial errors",
	}, []string{"scheme"})
)

// HostOption ...
type HostOption func(h *Host) error

// PeerHandler ...
type PeerHandler func(p *Peer)

// WithLabel ...
func WithLabel(label string) HostOption {
	return func(h *Host) error {
		h.label = label
		return nil
	}
}

// WithRateLimit ...
func WithRateLimit(limit uint64) HostOption {
	return func(h *Host) error {
		h.qos.SetRateLimit(limit)
		return nil
	}
}

// New ...
func New(logger *zap.Logger, profileKey *key.Key, options ...HostOption) (*Host, error) {
	hostKey, err := dao.GenerateKey()
	if err != nil {
		return nil, err
	}

	h := &Host{
		logger:     logger,
		profileKey: profileKey,
		key:        hostKey,
		peers:      map[kademlia.ID]*Peer{},
		qos:        qos.New(),
	}

	for _, o := range options {
		if err := o(h); err != nil {
			return nil, err
		}
	}

	for _, iface := range h.interfaces {
		if listener, ok := iface.(Listener); ok {
			go func() {
				if err := listener.Listen(h); err != nil {
					log.Println(err)
				}
			}()
		}
	}

	return h, nil
}

// Host ...
type Host struct {
	logger           *zap.Logger
	profileKey       *key.Key
	key              *key.Key
	label            string
	interfaces       []Interface
	peerHandlersLock sync.Mutex
	peerHandlers     []PeerHandler
	peersLock        sync.Mutex
	peers            map[kademlia.ID]*Peer
	qos              *qos.Control
}

// Close ...
func (h *Host) Close() {
	for i, iface := range h.interfaces {
		if listener, ok := iface.(Listener); ok {
			listener.Close()
		}
		h.interfaces[i] = nil
	}
	h.interfaces = h.interfaces[:0]

	h.peersLock.Lock()
	defer h.peersLock.Unlock()
	for k, p := range h.peers {
		p.Close()
		delete(h.peers, k)
	}
}

// ID ...
func (h *Host) ID() kademlia.ID {
	return kademlia.MustUnmarshalID(h.key.Public)
}

// Key ...
func (h *Host) Key() *key.Key {
	return h.key
}

// Cert ...
func (h *Host) Cert() (*certificate.Certificate, error) {
	profileCert, err := dao.NewSelfSignedCertificate(h.profileKey, certificate.KeyUsage_KEY_USAGE_SIGN, hostCertValidDuration)
	if err != nil {
		return nil, fmt.Errorf("generating init cert failed: %w", err)
	}

	csr, err := dao.NewCertificateRequest(h.key, certificate.KeyUsage_KEY_USAGE_SIGN, dao.WithSubject(h.label))
	if err != nil {
		return nil, err
	}
	cert, err := dao.SignCertificateRequest(csr, hostCertValidDuration, h.profileKey)
	if err != nil {
		return nil, err
	}
	cert.ParentOneof = &certificate.Certificate_Parent{Parent: profileCert}

	return cert, nil
}

// AddLink ...
func (h *Host) AddLink(c Link) {
	go func() {
		linksActive.Inc()
		defer linksActive.Dec()

		cert, err := h.Cert()
		if err != nil {
			h.logger.Error("peer init error", zap.Error(err))
			return
		}

		p, err := newPeer(h.logger, c, h.profileKey, cert)
		if err != nil {
			h.logger.Error("peer init error", zap.Error(err))
			return
		}

		logger := h.logger.With(
			logutil.ByteHex("peer", p.Certificate.Key),
			zap.Stringer("host", p.HostID()),
			zap.Stringer("type", reflect.TypeOf(c)),
			zap.Int("mtu", c.MTU()),
		)

		h.peersLock.Lock()
		_, found := h.peers[p.HostID()]
		if !found {
			h.peers[p.HostID()] = p
		}
		h.peersLock.Unlock()

		if found {
			p.Close()
			logger.Debug("closed duplicate link")
			return
		}

		logger.Debug("created peer")

		h.handlePeer(p)

		logger.Debug("running peer")
		p.run()

		h.peersLock.Lock()
		delete(h.peers, p.HostID())
		h.peersLock.Unlock()

		logger.Debug("closed peer")
	}()
}

// AddPeerHandler ...
func (h *Host) AddPeerHandler(fn PeerHandler) {
	h.peerHandlersLock.Lock()
	defer h.peerHandlersLock.Unlock()
	h.peerHandlers = append(h.peerHandlers, fn)
}

// HasPeer ...
func (h *Host) HasPeer(hostID kademlia.ID) bool {
	h.peersLock.Lock()
	_, ok := h.peers[hostID]
	h.peersLock.Unlock()
	return ok
}

// GetPeer ...
func (h *Host) GetPeer(hostID kademlia.ID) (*Peer, bool) {
	h.peersLock.Lock()
	p, ok := h.peers[hostID]
	h.peersLock.Unlock()
	return p, ok
}

// Peers ...
func (h *Host) Peers() []*Peer {
	h.peersLock.Lock()
	defer h.peersLock.Unlock()

	peers := make([]*Peer, 0, len(h.peers))
	for _, p := range h.peers {
		peers = append(peers, p)
	}
	return peers
}

// QOS ...
func (h *Host) QOS() *qos.Control {
	return h.qos
}

func (h *Host) handlePeer(p *Peer) {
	h.peerHandlersLock.Lock()
	defer h.peerHandlersLock.Unlock()

	for _, fn := range h.peerHandlers {
		fn(p)
	}
}

func (h *Host) dialer(scheme string) Interface {
	for _, i := range h.interfaces {
		if i.ValidScheme(scheme) {
			return i
		}
	}
	return nil
}

// Dial ...
func (h *Host) Dial(addr InterfaceAddr) error {
	scheme := addr.Scheme()

	dialCount.WithLabelValues(scheme).Inc()
	h.logger.Debug(
		"dialing",
		zap.String("scheme", scheme),
		zap.Stringer("addr", addr.(fmt.Stringer)),
	)

	d := h.dialer(scheme)
	if d == nil {
		dialErrorCount.WithLabelValues(scheme).Inc()
		return errors.New("unsupported scheme")
	}

	c, err := d.Dial(addr)
	if err != nil {
		dialErrorCount.WithLabelValues(scheme).Inc()
		h.logger.Error("dial error", zap.Error(err))
		return err
	}

	h.AddLink(c)
	return nil
}

// Listener ...
type Listener interface {
	Listen(h *Host) error
	Close() error
}

// WithInterface ...
func WithInterface(i Interface) HostOption {
	return func(host *Host) error {
		host.interfaces = append(host.interfaces, i)
		return nil
	}
}

// Interface ...
type Interface interface {
	ValidScheme(string) bool
	Dial(addr InterfaceAddr) (Link, error)
}

// InterfaceAddr ...
type InterfaceAddr interface {
	Scheme() string
}

// Link ...
type Link interface {
	io.ReadWriteCloser
	MTU() int
}
