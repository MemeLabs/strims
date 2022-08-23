// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build !js

package vnic

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"sync"

	vnicv1 "github.com/MemeLabs/strims/pkg/apis/vnic/v1"
	"github.com/MemeLabs/strims/pkg/pointerutil"
	"github.com/pion/ice/v2"
	"github.com/pion/logging"
	"github.com/pion/webrtc/v3"
	"go.uber.org/zap"
	"golang.org/x/exp/slices"
)

func init() { RegisterLinkInterface("webrtc", (*webRTCLinkCandidate)(nil)) }

var _ Interface = (*webRTCInterface)(nil)
var _ LinkCandidate = (*webRTCLinkCandidate)(nil)

// WebRTCInterfaceOptions ...
type WebRTCInterfaceOptions struct {
	ICEServers    []string
	PortMin       uint16
	PortMax       uint16
	UDPMux        ice.UDPMux
	TCPMux        ice.TCPMux
	HostIP        string
	EnableLogging bool
}

func NewWebRTCUDPMux(address string) (ice.UDPMux, *net.UDPConn, error) {
	addr, err := net.ResolveUDPAddr("udp", address)
	if err != nil {
		return nil, nil, err
	}
	lis, err := net.ListenUDP("udp", addr)
	if err != nil {
		return nil, nil, fmt.Errorf("opening udp listener: %w", err)
	}
	return webrtc.NewICEUDPMux(nil, lis), lis, nil
}

func NewWebRTCTCPMux(address string, readBufferSize int) (ice.TCPMux, *net.TCPListener, error) {
	addr, err := net.ResolveTCPAddr("tcp", address)
	if err != nil {
		return nil, nil, err
	}
	lis, err := net.ListenTCP("tcp", addr)
	if err != nil {
		return nil, nil, fmt.Errorf("opening tcp listener: %w", err)
	}
	return webrtc.NewICETCPMux(nil, lis, readBufferSize), lis, nil
}

var DefaultWebRTCICEServers = []string{
	"stun:stun.l.google.com:19302",
	"stun:stun1.l.google.com:19302",
	"stun:stun2.l.google.com:19302",
	"stun:stun3.l.google.com:19302",
	"stun:stun4.l.google.com:19302",
}

// NewWebRTCInterface ...
func NewWebRTCInterface(logger *zap.Logger, o *WebRTCInterfaceOptions) Interface {
	if o == nil {
		o = &WebRTCInterfaceOptions{}
	}

	s := webrtc.SettingEngine{}

	networkTypes := []webrtc.NetworkType{webrtc.NetworkTypeUDP4, webrtc.NetworkTypeUDP6}
	if o.UDPMux != nil {
		s.SetICEUDPMux(o.UDPMux)
	} else {
		s.SetEphemeralUDPPortRange(o.PortMin, o.PortMax)
	}
	if o.TCPMux != nil {
		s.SetICETCPMux(o.TCPMux)
		networkTypes = append(networkTypes, webrtc.NetworkTypeTCP4, webrtc.NetworkTypeTCP6)
	}
	s.SetNetworkTypes(networkTypes)

	if o.HostIP != "" {
		s.SetNAT1To1IPs([]string{o.HostIP}, webrtc.ICECandidateTypeHost)
	}

	if o.EnableLogging {
		s.LoggerFactory = &pionLoggerFactory{logger}
	}

	s.SetSCTPMaxReceiveBufferSize(8 * 1024 * 1024)
	s.DetachDataChannels()

	return &webRTCInterface{
		logger:  logger,
		options: o,
		api:     webrtc.NewAPI(webrtc.WithSettingEngine(s)),
	}
}

// webRTCInterface ...
type webRTCInterface struct {
	logger  *zap.Logger
	options *WebRTCInterfaceOptions
	api     *webrtc.API
}

// CreateLinkCandidate ...
func (d *webRTCInterface) CreateLinkCandidate(ctx context.Context, h *Host) (LinkCandidate, error) {
	iceServers := d.options.ICEServers
	if iceServers == nil {
		iceServers = slices.Clone(DefaultWebRTCICEServers)
	}
	config := webrtc.Configuration{
		ICEServers: []webrtc.ICEServer{
			{
				URLs: iceServers,
			},
		},
	}

	pc, err := d.api.NewPeerConnection(config)
	if err != nil {
		return nil, err
	}

	done := make(chan struct{})
	var doneOnce sync.Once

	go func() {
		select {
		case <-done:
		case <-ctx.Done():
			pc.Close()
		}
	}()

	pc.OnConnectionStateChange(func(s webrtc.PeerConnectionState) {
		d.logger.Debug("connection state changed", zap.String("state", s.String()))
		switch s {
		case webrtc.PeerConnectionStateDisconnected:
		case webrtc.PeerConnectionStateFailed:
			pc.Close()
		case webrtc.PeerConnectionStateClosed:
			doneOnce.Do(func() { close(done) })
		}
	})

	dc, err := pc.CreateDataChannel("data", &webrtc.DataChannelInit{
		ID:         pointerutil.To[uint16](1),
		Negotiated: pointerutil.To(true),
	})
	if err != nil {
		return nil, err
	}
	dc.OnOpen(func() {
		d.logger.Debug("DataChannel opened")
		doneOnce.Do(func() { close(done) })
	})

	c := &webRTCLinkCandidate{
		ctx:  ctx,
		host: h,
		pc:   pc,
		dc:   dc,
		done: done,
	}
	return c, nil
}

type webRTCLinkCandidate struct {
	ctx       context.Context
	host      *Host
	pc        *webrtc.PeerConnection
	dc        *webrtc.DataChannel
	done      chan struct{}
	haveDesc  bool
	localDesc bool
}

func (c *webRTCLinkCandidate) LocalDescription() (d *vnicv1.LinkDescription, err error) {
	var desc webrtc.SessionDescription
	if c.haveDesc {
		desc, err = c.pc.CreateAnswer(nil)
	} else {
		desc, err = c.pc.CreateOffer(nil)
	}
	if err != nil {
		return nil, err
	}

	if err := c.pc.SetLocalDescription(desc); err != nil {
		return nil, err
	}

	<-webrtc.GatheringCompletePromise(c.pc)

	sdp, err := json.Marshal(c.pc.LocalDescription())
	if err != nil {
		return nil, err
	}

	d = &vnicv1.LinkDescription{
		Interface:   "webrtc",
		Description: string(sdp),
	}
	return d, nil
}

func (c *webRTCLinkCandidate) SetRemoteDescription(d *vnicv1.LinkDescription) (bool, error) {
	var desc webrtc.SessionDescription
	if err := json.Unmarshal([]byte(d.Description), &desc); err != nil {
		return false, err
	}

	if err := c.pc.SetRemoteDescription(desc); err != nil {
		return false, err
	}
	c.haveDesc = true

	if c.localDesc {
		return c.awaitLink()
	}
	go c.awaitLink()
	return false, nil
}

func (c *webRTCLinkCandidate) awaitLink() (bool, error) {
	select {
	case <-c.ctx.Done():
		return false, c.ctx.Err()
	case <-c.done:
		rwc, err := c.dc.Detach()
		if err != nil {
			return false, err
		}

		c.host.AddLink(newDCLink(rwc))
		return true, nil
	}
}

const dcMTU = 64 * 1024

func newDCLink(rwc io.ReadWriteCloser) *dcLink {
	return &dcLink{
		Reader:      bufio.NewReaderSize(rwc, dcMTU),
		WriteCloser: rwc,
	}
}

type dcLink struct {
	io.Reader
	io.WriteCloser
}

func (l dcLink) MTU() int {
	return dcMTU
}

type pionLoggerFactory struct {
	logger *zap.Logger
}

func (f *pionLoggerFactory) NewLogger(scope string) logging.LeveledLogger {
	return pionLeveledLogger{f.logger.With(zap.String("scope", scope)).Sugar()}
}

type pionLeveledLogger struct {
	*zap.SugaredLogger
}

func (l pionLeveledLogger) Trace(msg string) {
	l.SugaredLogger.Debug(msg)
}

func (l pionLeveledLogger) Tracef(format string, args ...interface{}) {
	l.SugaredLogger.Debugf(format, args...)
}

func (l pionLeveledLogger) Debug(msg string) {
	l.SugaredLogger.Debug(msg)
}

func (l pionLeveledLogger) Info(msg string) {
	l.SugaredLogger.Info(msg)
}

func (l pionLeveledLogger) Warn(msg string) {
	l.SugaredLogger.Warn(msg)
}

func (l pionLeveledLogger) Error(msg string) {
	l.SugaredLogger.Error(msg)
}
