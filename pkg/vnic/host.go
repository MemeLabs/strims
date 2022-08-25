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
	"time"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	vnicv1 "github.com/MemeLabs/strims/pkg/apis/vnic/v1"
	"github.com/MemeLabs/strims/pkg/errutil"
	"github.com/MemeLabs/strims/pkg/kademlia"
	"github.com/MemeLabs/strims/pkg/logutil"
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
	peersLock        sync.Mutex
	peers            map[kademlia.ID]*Peer
	maxPeers         int
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

// AddLink ...
func (h *Host) AddLink(c Link) {
	go func() {
		linksActive.Inc()
		defer linksActive.Dec()

		defer c.Close()

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

// PeerCount ...
func (h *Host) PeerCount() int {
	h.peersLock.Lock()
	defer h.peersLock.Unlock()
	return len(h.peers)
}

// SetMaxPeers ...
func (h *Host) SetMaxPeers(n int) {
	h.peersLock.Lock()
	defer h.peersLock.Unlock()

	if n <= 0 {
		n = math.MaxInt
	}
	h.maxPeers = n
}

// MaxPeers ...
func (h *Host) MaxPeers() int {
	h.peersLock.Lock()
	defer h.peersLock.Unlock()
	return h.maxPeers
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
func (h *Host) Dial(uri string) error {
	u, err := url.Parse(uri)
	if err != nil {
		return err
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
		return errors.New("unsupported scheme")
	}

	c, err := d.Dial(uri)
	if err != nil {
		dialErrorCount.WithLabelValues(u.Scheme).Inc()
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
