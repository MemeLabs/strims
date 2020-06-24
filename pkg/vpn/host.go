package vpn

import (
	"encoding/binary"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"math"
	"path"
	"reflect"
	"runtime"
	"sync"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/petar/GoLLRB/llrb"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
	"go.uber.org/zap"
)

const reservedPortCount uint16 = 1000

// default network service ports
const (
	HashTablePort uint16 = iota + 10
	PeerIndexPort
	PeerExchangePort
	DirectoryPort
	SwarmServicePort
)

// peer link ports
const (
	NetworkInitPort uint16 = iota
	NetworkBrokerPort
	BootstrapPort
	SwarmPort
)

var (
	linksActive = promauto.NewGauge(prometheus.GaugeOpts{
		Name: "strims_vpn_links_active",
		Help: "The number of active network links",
	})
	linkReadBytes = promauto.NewCounter(prometheus.CounterOpts{
		Name: "strims_vpn_link_read_bytes",
		Help: "The total number of bytes read from network links",
	})
	linkWriteBytes = promauto.NewCounter(prometheus.CounterOpts{
		Name: "strims_vpn_link_write_bytes",
		Help: "The total number of bytes written to network links",
	})
	dialCount = promauto.NewCounterVec(prometheus.CounterOpts{
		Name: "strims_vpn_dial_count",
		Help: "The total number of dialed network connections",
	}, []string{"scheme"})
	dialErrorCount = promauto.NewCounterVec(prometheus.CounterOpts{
		Name: "strims_vpn_dial_error_count",
		Help: "The total number of network connection dial errors",
	}, []string{"scheme"})
)

// HostOption ...
type HostOption func(h *Host) error

// PeerHandler ...
type PeerHandler func(p *Peer)

// NewHost ...
func NewHost(logger *zap.Logger, key *pb.Key, options ...HostOption) (*Host, error) {
	discriminator, err := randUint16(math.MaxUint16)
	if err != nil {
		return nil, err
	}

	id, err := NewHostID(key.Public, discriminator)
	if err != nil {
		return nil, err
	}

	h := &Host{
		logger:        logger,
		discriminator: discriminator,
		id:            id,
		key:           key,
	}

	for _, o := range options {
		if err := o(h); err != nil {
			return nil, err
		}
	}

	for _, iface := range h.interfaces {
		if listener, ok := iface.(Listener); ok {
			go listener.Listen(h)
		}
	}

	return h, nil
}

// Host ...
type Host struct {
	logger           *zap.Logger
	discriminator    uint16
	id               kademlia.ID
	key              *pb.Key
	interfaces       []Interface
	peerHandlersLock sync.Mutex
	peerHandlers     []PeerHandler
	peersLock        sync.Mutex
	peers            peerMap

	// TODO: find a better place for this...
	networkBroker NetworkBroker
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

// Logger ...
func (h *Host) Logger() *zap.Logger {
	return h.logger
}

// Discriminator ...
func (h *Host) Discriminator() uint16 {
	return h.discriminator
}

// ID ...
func (h *Host) ID() kademlia.ID {
	return h.id
}

// Key ...
func (h *Host) Key() *pb.Key {
	return h.key
}

// AddLink ...
func (h *Host) AddLink(c Link) {
	go func() {
		linksActive.Inc()
		defer linksActive.Dec()

		p, err := newPeer(h.logger, instrumentLink(c), h.key, h.id)
		if err != nil {
			h.logger.Error("peer init error", zap.Error(err))
			return
		}

		h.logger.Debug(
			"created peer",
			logutil.ByteHex("peer", p.Certificate.Key),
			zap.String("type", reflect.TypeOf(c).String()),
			zap.Int("mtu", c.MTU()),
		)

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

// NewHostID ...
func NewHostID(key []byte, hid uint16) (kademlia.ID, error) {
	var t [20]byte
	copy(t[:18], key)
	binary.BigEndian.PutUint16(t[18:], hid)
	return kademlia.UnmarshalID(t[:])
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

func jsonDump(i interface{}) {
	_, file, line, _ := runtime.Caller(1)
	b, err := json.MarshalIndent(i, "", "  ")
	if err != nil {
		panic(err)
	}
	fmt.Printf(
		"%s %s:%d: %s\n",
		time.Now().Format("2006/01/02 15:04:05.000000"),
		path.Base(file),
		line, string(b),
	)
}
