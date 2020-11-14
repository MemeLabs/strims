package vnic

import (
	"errors"
	"fmt"
	"math"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/protoutil"
	"github.com/MemeLabs/go-ppspp/pkg/randutil"
	"github.com/MemeLabs/go-ppspp/pkg/version"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
	"go.uber.org/zap"
)

var (
	frameReadCount = promauto.NewCounter(prometheus.CounterOpts{
		Name: "strims_vpn_frame_read_count",
		Help: "The total number of frames read",
	})
	frameReadBytes = promauto.NewCounter(prometheus.CounterOpts{
		Name: "strims_vpn_frame_read_bytes",
		Help: "The total number of frame bytes read",
	})
	frameHandlerNotFoundCount = promauto.NewCounter(prometheus.CounterOpts{
		Name: "strims_vpn_frame_hander_not_found_count",
		Help: "The total number of unhandled frames",
	})
	frameHandlerErrorCount = promauto.NewCounter(prometheus.CounterOpts{
		Name: "strims_vpn_frame_handler_error_count",
		Help: "The total number of frame handler errors",
	})
)

func newPeer(logger *zap.Logger, link Link, hostKey *pb.Key, hostCert *pb.Certificate) (*Peer, error) {
	err := protoutil.WriteStream(link, &pb.PeerInit{
		ProtocolVersion: 1,
		Certificate:     hostCert,
		NodePlatform:    version.Platform,
		NodeVersion:     version.Version,
	})
	if err != nil {
		return nil, fmt.Errorf("writing peer init failed: %w", err)
	}

	var init pb.PeerInit
	if err = protoutil.ReadStream(link, &init); err != nil {
		return nil, fmt.Errorf("reading peer init failed: %w", err)
	}

	if err := dao.VerifyCertificate(init.Certificate); err != nil {
		return nil, fmt.Errorf("peer cert verification failed: %w", err)
	}
	if init.Certificate.GetParent() == nil {
		return nil, errors.New("invalid peer certificate")
	}

	hostID, err := kademlia.UnmarshalID(init.Certificate.Key)
	if err != nil {
		return nil, fmt.Errorf("peer host id malformed: %w", err)
	}

	p := &Peer{
		logger:       logger,
		Link:         link,
		Certificate:  init.Certificate.GetParent(),
		hostID:       hostID,
		handlers:     map[uint16]FrameHandler{},
		reservations: map[uint16]struct{}{},
		channels:     map[uint16]*FrameReadWriter{},
		done:         make(chan struct{}),
	}
	return p, nil
}

// Peer ...
type Peer struct {
	logger           *zap.Logger
	Link             Link
	Certificate      *pb.Certificate
	hostID           kademlia.ID
	handlersLock     sync.Mutex
	handlers         map[uint16]FrameHandler
	reservationsLock sync.Mutex
	reservations     map[uint16]struct{}
	channelsLock     sync.Mutex
	channels         map[uint16]*FrameReadWriter
	done             chan struct{}
	closeOnce        sync.Once
}

func (p *Peer) run() {
	for {
		var f Frame
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
		close(p.done)
		p.Link.Close()

		p.channelsLock.Lock()
		defer p.channelsLock.Unlock()
		for port, ch := range p.channels {
			delete(p.channels, port)
			ch.Close()
		}
	})
}

// Done ...
func (p *Peer) Done() <-chan struct{} {
	return p.done
}

// HostID ...
func (p *Peer) HostID() kademlia.ID {
	return p.hostID
}

// SetHandler ...
func (p *Peer) SetHandler(port uint16, h FrameHandler) {
	p.handlersLock.Lock()
	p.reservationsLock.Lock()
	defer p.reservationsLock.Unlock()
	defer p.handlersLock.Unlock()

	p.reservations[port] = struct{}{}
	p.handlers[port] = h
}

// RemoveHandler ...
func (p *Peer) RemoveHandler(port uint16) {
	p.handlersLock.Lock()
	p.reservationsLock.Lock()
	defer p.reservationsLock.Unlock()
	defer p.handlersLock.Unlock()

	delete(p.handlers, port)
	delete(p.reservations, port)
}

// Handler ...
func (p *Peer) Handler(port uint16) FrameHandler {
	p.handlersLock.Lock()
	defer p.handlersLock.Unlock()
	return p.handlers[port]
}

// ReservePort ...
func (p *Peer) ReservePort() (uint16, error) {
	p.reservationsLock.Lock()
	defer p.reservationsLock.Unlock()

	for {
		port, err := randutil.Uint16n(math.MaxUint16 - reservedPortCount)
		if err != nil {
			return 0, err
		}
		port += reservedPortCount

		if _, ok := p.reservations[port]; !ok {
			p.reservations[port] = struct{}{}
			return port, nil
		}
	}
}

// ReleasePort ...
func (p *Peer) ReleasePort(port uint16) {
	p.reservationsLock.Lock()
	defer p.reservationsLock.Unlock()

	delete(p.reservations, port)
}

// Channel ...
func (p *Peer) Channel(port uint16) *FrameReadWriter {
	f := NewFrameReadWriter(p.Link, port)
	p.SetHandler(port, f.HandleFrame)

	p.channelsLock.Lock()
	defer p.channelsLock.Unlock()
	p.channels[port] = f

	return f
}

// CloseChannel ...
func (p *Peer) CloseChannel(f *FrameReadWriter) error {
	p.RemoveHandler(f.port)

	p.channelsLock.Lock()
	defer p.channelsLock.Unlock()
	delete(p.channels, f.port)

	return f.Close()
}

// FrameHandler ...
type FrameHandler func(p *Peer, f Frame) error
