// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package vnic

import (
	"context"
	"errors"
	"fmt"
	"io"
	"math"
	"net/url"
	"reflect"
	"sync"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	vnicv1 "github.com/MemeLabs/strims/pkg/apis/vnic/v1"
	"github.com/MemeLabs/strims/pkg/errutil"
	"github.com/MemeLabs/strims/pkg/kademlia"
	"github.com/MemeLabs/strims/pkg/syncutil"
	"github.com/MemeLabs/strims/pkg/vnic/qos"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
	"go.uber.org/multierr"
	"go.uber.org/zap"
)

const reservedPortCount uint16 = 1000
const hostCertValidDuration = time.Hour

// default network service ports
const (
	HashTablePort uint16 = iota + 10
	PeerIndexPort
	PeerExchangePort
	TransferPort
	SnippetPort
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
					logger.Warn("interface listener closed", zap.Error(err))
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
	peers            syncutil.Map[kademlia.ID, *Peer]
	maxPeers         atomic.Int64
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

	h.peers.Each(func(_ kademlia.ID, p *Peer) { p.Close() })
	h.peers.Clear()
}

func (h *Host) LinkCandidates(ctx context.Context) (*LinkCandidatePool, error) {
	var cs []LinkCandidate
	var errs []error
	for _, i := range h.interfaces {
		c, err := i.CreateLinkCandidate(ctx, h)
		if err != nil {
			errs = append(errs, err)
		} else {
			cs = append(cs, c)
		}
	}
	if len(cs) == 0 {
		return nil, multierr.Combine(errs...)
	}
	return &LinkCandidatePool{cs}, nil
}

// ID ...
func (h *Host) ID() kademlia.ID {
	return errutil.Must(kademlia.UnmarshalID(h.key.Public))
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

func (h *Host) AddLink(c Link) (*Peer, error) {
	logger := h.logger.With(
		zap.Stringer("type", reflect.TypeOf(c)),
		zap.Int("mtu", c.MTU()),
	)

	p, err := h.addLink(logger, c)
	if err != nil {
		logger.Warn("peer init failed", zap.Error(err))
		c.Close()
	}
	return p, nil
}

func (h *Host) addLink(logger *zap.Logger, c Link) (*Peer, error) {
	cert, err := h.Cert()
	if err != nil {
		return nil, &PeerInitError{err}
	}

	p, err := newPeer(logger, c, h.profileKey, cert)
	if err != nil {
		return nil, &PeerInitError{err}
	}

	_, found := h.peers.GetOrInsert(p.HostID(), p)
	if found {
		return nil, &PeerInitError{errors.New("duplicate peer link found")}
	}

	go func() {
		linksActive.Inc()
		defer linksActive.Dec()

		h.handlePeer(p)

		p.run()

		h.peers.Delete(p.HostID())
	}()

	return p, nil
}

// AddPeerHandler ...
func (h *Host) AddPeerHandler(fn PeerHandler) {
	h.peerHandlersLock.Lock()
	defer h.peerHandlersLock.Unlock()
	h.peerHandlers = append(h.peerHandlers, fn)
}

// HasPeer ...
func (h *Host) HasPeer(hostID kademlia.ID) bool {
	return h.peers.Has(hostID)
}

// GetPeer ...
func (h *Host) GetPeer(hostID kademlia.ID) (*Peer, bool) {
	return h.peers.Get(hostID)
}

// Peers ...
func (h *Host) Peers() []*Peer {
	return h.peers.Values()
}

// PeerCount ...
func (h *Host) PeerCount() int {
	return h.peers.Len()
}

// SetMaxPeers ...
func (h *Host) SetMaxPeers(n int) {
	if n <= 0 {
		n = math.MaxInt32
	}
	h.maxPeers.Store(int64(n))
}

// MaxPeers ...
func (h *Host) MaxPeers() int {
	return int(h.maxPeers.Load())
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

func (h *Host) dialer(scheme string) LinkDialer {
	for _, ii := range h.interfaces {
		if i, ok := ii.(LinkDialer); ok {
			if i.ValidScheme(scheme) {
				return i
			}
		}
	}
	return nil
}

// Dial ...
func (h *Host) Dial(uri string) (*Peer, error) {
	u, err := url.Parse(uri)
	if err != nil {
		return nil, err
	}

	dialCount.WithLabelValues(u.Scheme).Inc()
	h.logger.Debug(
		"dialing",
		zap.String("scheme", u.Scheme),
		zap.String("uri", uri),
	)

	d := h.dialer(u.Scheme)
	if d == nil {
		dialErrorCount.WithLabelValues(u.Scheme).Inc()
		return nil, errors.New("unsupported scheme")
	}

	c, err := d.Dial(uri)
	if err != nil {
		dialErrorCount.WithLabelValues(u.Scheme).Inc()
		h.logger.Error("dial error", zap.Error(err))
		return nil, err
	}

	return h.AddLink(c)
}

type PeerInitError struct {
	error
}

func (e *PeerInitError) Unwrap() error {
	return e.error
}

func (e *PeerInitError) Error() string {
	return fmt.Sprintf("peer init: %s", e.error)
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

type Interface interface {
	CreateLinkCandidate(ctx context.Context, h *Host) (LinkCandidate, error)
}

type LinkDialer interface {
	ValidScheme(scheme string) bool
	Dial(addr string) (Link, error)
}

type LinkCandidate interface {
	LocalDescription() (*vnicv1.LinkDescription, error)
	SetRemoteDescription(d *vnicv1.LinkDescription) (bool, error)
}

// Link ...
type Link interface {
	io.ReadWriteCloser
	MTU() int
}
