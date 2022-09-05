// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package vnic

import (
	"context"
	"errors"
	"fmt"
	"math"
	"sync"
	"sync/atomic"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	vnicv1 "github.com/MemeLabs/strims/pkg/apis/vnic/v1"
	"github.com/MemeLabs/strims/pkg/kademlia"
	"github.com/MemeLabs/strims/pkg/logutil"
	"github.com/MemeLabs/strims/pkg/protoutil"
	"github.com/MemeLabs/strims/pkg/randutil"
	"github.com/MemeLabs/strims/pkg/version"
	"github.com/MemeLabs/strims/pkg/vnic/qos"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
	"go.uber.org/zap"
)

var (
	frameReadCount = promauto.NewCounter(prometheus.CounterOpts{
		Name: "strims_vnic_frame_read_count",
		Help: "The total number of frames read",
	})
	frameReadBytes = promauto.NewCounter(prometheus.CounterOpts{
		Name: "strims_vnic_frame_read_bytes",
		Help: "The total number of frame bytes read",
	})
	frameHandlerNotFoundCount = promauto.NewCounter(prometheus.CounterOpts{
		Name: "strims_vnic_frame_hander_not_found_count",
		Help: "The total number of unhandled frames",
	})
	frameHandlerErrorCount = promauto.NewCounter(prometheus.CounterOpts{
		Name: "strims_vnic_frame_handler_error_count",
		Help: "The total number of frame handler errors",
	})
)

func newPeer(logger *zap.Logger, link Link, hostKey *key.Key, hostCert *certificate.Certificate) (*Peer, error) {
	err := protoutil.WriteStream(link, &vnicv1.PeerInit{
		ProtocolVersion: 1,
		Certificate:     hostCert,
		NodePlatform:    version.Platform,
		NodeVersion:     version.Version,
	})
	if err != nil {
		return nil, fmt.Errorf("writing peer init: %w", err)
	}

	var init vnicv1.PeerInit
	if err = protoutil.ReadStream(link, &init); err != nil {
		return nil, fmt.Errorf("reading peer init: %w", err)
	}

	if err := dao.VerifyCertificate(init.Certificate); err != nil {
		return nil, fmt.Errorf("peer cert verification: %w", err)
	}

	peerCert := init.Certificate.GetParent()
	if peerCert == nil {
		return nil, errors.New("invalid peer certificate")
	}

	hostID, err := kademlia.UnmarshalID(init.Certificate.Key)
	if err != nil {
		return nil, fmt.Errorf("peer host id malformed: %w", err)
	}

	ctx, cancel := context.WithCancel(context.Background())

	p := &Peer{
		logger: logger.With(
			logutil.ByteHex("peer", peerCert.Key),
			zap.Stringer("host", hostID),
		),
		Link:         instrumentLink(link, hostID),
		Certificate:  peerCert,
		hostID:       hostID,
		handlers:     map[uint16]FrameHandler{},
		reservations: map[uint16]struct{}{},
		channels:     map[uint16]*FrameReadWriter{},
		ctx:          ctx,
		close:        cancel,
	}
	return p, nil
}

// Peer ...
type Peer struct {
	logger           *zap.Logger
	Link             Link
	Certificate      *certificate.Certificate
	hostID           kademlia.ID
	handlersLock     sync.Mutex
	handlers         map[uint16]FrameHandler
	reservationsLock sync.Mutex
	reservations     map[uint16]struct{}
	channelsLock     sync.Mutex
	channels         map[uint16]*FrameReadWriter
	ctx              context.Context
	close            context.CancelFunc
	closeOnce        sync.Once
	closed           uint32
}

func (p *Peer) run() {
	p.logger.Debug("running peer")

	var f Frame
	for {
		if _, err := f.ReadFrom(p.Link); err != nil {
			p.logger.Info("failed to read frame", zap.Error(err))
			break
		}
		frameReadCount.Inc()
		frameReadBytes.Add(float64(len(f.Body)))

		h := p.Handler(f.Header.Port)
		if h == nil {
			frameHandlerNotFoundCount.Inc()
			continue
		}

		if err := h(p, f); err != nil {
			p.logger.Warn("failed to run frame handler", zap.Error(err))
			frameHandlerErrorCount.Inc()
		}

		f.Free()
	}

	p.Close()
}

// Close ...
func (p *Peer) Close() {
	p.closeOnce.Do(func() {
		p.logger.Debug("closing peer")

		atomic.StoreUint32(&p.closed, 1)
		p.close()
		deleteInstrumentedLinkMetrics(p.Link, p.hostID)

		p.channelsLock.Lock()
		defer p.channelsLock.Unlock()
		for port, ch := range p.channels {
			delete(p.channels, port)
			ch.Close()
		}

		p.Link.Close()
	})
}

// Context ...
func (p *Peer) Context() context.Context {
	return p.ctx
}

// Done ...
func (p *Peer) Done() <-chan struct{} {
	return p.ctx.Done()
}

// Closed ...
func (p *Peer) Closed() bool {
	return atomic.LoadUint32(&p.closed) == 1
}

// HostID ...
func (p *Peer) HostID() kademlia.ID {
	return p.hostID
}

// SetHandler ...
func (p *Peer) SetHandler(port uint16, h FrameHandler) {
	p.handlersLock.Lock()
	defer p.handlersLock.Unlock()

	p.handlers[port] = h
}

// RemoveHandler ...
func (p *Peer) RemoveHandler(port uint16) {
	p.handlersLock.Lock()
	defer p.handlersLock.Unlock()

	delete(p.handlers, port)
}

// Handler ...
func (p *Peer) Handler(port uint16) FrameHandler {
	p.handlersLock.Lock()
	defer p.handlersLock.Unlock()
	return p.handlers[port]
}

// ReservePort ...
func (p *Peer) ReservePort() (uint16, error) {
	p.handlersLock.Lock()
	p.reservationsLock.Lock()
	defer p.handlersLock.Unlock()
	defer p.reservationsLock.Unlock()

	for {
		port, err := randutil.Uint16n(math.MaxUint16 - reservedPortCount)
		if err != nil {
			return 0, err
		}
		port += reservedPortCount

		if _, ok := p.handlers[port]; ok {
			continue
		}
		if _, ok := p.reservations[port]; ok {
			continue
		}

		p.reservations[port] = struct{}{}
		return port, nil
	}
}

// ReleasePort ...
func (p *Peer) ReleasePort(port uint16) {
	p.reservationsLock.Lock()
	defer p.reservationsLock.Unlock()

	delete(p.reservations, port)
}

// Channel ...
func (p *Peer) Channel(port uint16, qc *qos.Class) *FrameReadWriter {
	f := NewFrameReadWriter(p.Link, port, qc)
	p.SetHandler(port, f.HandleFrame)

	p.channelsLock.Lock()
	defer p.channelsLock.Unlock()
	p.channels[port] = f

	return f
}

// ChannelPair creates a symmetric channel pair.
func (p *Peer) ChannelPair(port0, port1 uint16, qc *qos.Class) (rw0, rw1 *FrameReadWriter) {
	rw0 = NewFrameReadWriter(p.Link, port0, qc)
	rw1 = NewFrameReadWriter(p.Link, port1, qc)
	p.SetHandler(port1, rw0.HandleFrame)
	p.SetHandler(port0, rw1.HandleFrame)

	p.channelsLock.Lock()
	defer p.channelsLock.Unlock()
	p.channels[port0] = rw0
	p.channels[port1] = rw1

	return
}

// CloseChannel ...
func (p *Peer) CloseChannel(f *FrameReadWriter) error {
	p.RemoveHandler(f.Port())

	p.channelsLock.Lock()
	defer p.channelsLock.Unlock()
	delete(p.channels, f.Port())

	return f.Close()
}

// FrameHandler ...
type FrameHandler func(p *Peer, f Frame) error
