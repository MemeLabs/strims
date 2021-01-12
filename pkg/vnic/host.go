package vnic

import (
	"errors"
	"fmt"
	"io"
	"log"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/petar/GoLLRB/llrb"
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
	linkReadBytes = promauto.NewCounter(prometheus.CounterOpts{
		Name: "strims_vnic_link_read_bytes",
		Help: "The total number of bytes read from network links",
	})
	linkWriteBytes = promauto.NewCounter(prometheus.CounterOpts{
		Name: "strims_vnic_link_write_bytes",
		Help: "The total number of bytes written to network links",
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
	peers            peerMap
}

// Close ...
func (h *Host) Close() {
	for _, iface := range h.interfaces {
		if listener, ok := iface.(Listener); ok {
			listener.Close()
		}
	}

	h.peersLock.Lock()
	defer h.peersLock.Unlock()
	h.peers.Each(func(p *Peer) bool {
		p.Close()
		return true
	})
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

		p, err := newPeer(h.logger, instrumentLink(c), h.profileKey, cert)
		if err != nil {
			h.logger.Error("peer init error", zap.Error(err))
			return
		}

		// h.logger.Debug(
		// 	"created peer",
		// 	logutil.ByteHex("peer", p.Certificate.Key),
		// 	zap.String("type", reflect.TypeOf(c).String()),
		// 	zap.Int("mtu", c.MTU()),
		// )

		h.handlePeer(p)

		h.peersLock.Lock()
		h.peers.Insert(p.HostID(), p)
		h.peersLock.Unlock()

		p.run()

		h.peersLock.Lock()
		h.peers.Delete(p.HostID())
		h.peersLock.Unlock()
	}()
}

// AddPeerHandler ...
func (h *Host) AddPeerHandler(fn PeerHandler) {
	h.peerHandlersLock.Lock()
	defer h.peerHandlersLock.Unlock()
	h.peerHandlers = append(h.peerHandlers, fn)
}

// GetPeer ...
func (h *Host) GetPeer(hostID kademlia.ID) (*Peer, bool) {
	h.peersLock.Lock()
	defer h.peersLock.Unlock()
	return h.peers.Get(hostID)
}

// Peers ...
func (h *Host) Peers() []*Peer {
	h.peersLock.Lock()
	defer h.peersLock.Unlock()

	peers := []*Peer{}
	h.peers.Each(func(p *Peer) bool {
		peers = append(peers, p)
		return true
	})
	return peers
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

	if err := d.Dial(h, addr); err != nil {
		dialErrorCount.WithLabelValues(scheme).Inc()
		h.logger.Error("dial error", zap.Error(err))
		return err
	}
	return nil
}

type peerMap struct {
	m llrb.LLRB
}

func (m *peerMap) Insert(k kademlia.ID, v *Peer) {
	m.m.InsertNoReplace(peerMapItem{k, v})
}

func (m *peerMap) Delete(k kademlia.ID) {
	m.m.Delete(peerMapItem{k, nil})
}

func (m *peerMap) Get(k kademlia.ID) (*Peer, bool) {
	if it := m.m.Get(peerMapItem{k, nil}); it != nil {
		return it.(peerMapItem).v, true
	}
	return nil, false
}

func (m *peerMap) Each(f func(b *Peer) bool) {
	m.m.AscendGreaterOrEqual(llrb.Inf(-1), func(i llrb.Item) bool {
		return f(i.(peerMapItem).v)
	})
}

type peerMapItem struct {
	k kademlia.ID
	v *Peer
}

func (t peerMapItem) Less(oi llrb.Item) bool {
	if o, ok := oi.(peerMapItem); ok {
		return t.k.Less(o.k)
	}
	return !oi.Less(t)
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
	Dial(h *Host, addr InterfaceAddr) error
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

func instrumentLink(l Link) *instrumentedLink {
	return &instrumentedLink{l}
}

type instrumentedLink struct {
	Link
}

func (l *instrumentedLink) Read(p []byte) (int, error) {
	n, err := l.Link.Read(p)
	linkReadBytes.Add(float64(n))
	return n, err
}

func (l *instrumentedLink) Write(p []byte) (int, error) {
	n, err := l.Link.Write(p)
	linkWriteBytes.Add(float64(n))
	return n, err
}
