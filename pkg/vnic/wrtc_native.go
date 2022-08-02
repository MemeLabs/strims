// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build !js

package vnic

import (
	"bufio"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net"
	"strconv"
	"time"

	"github.com/pion/ice/v2"
	"github.com/pion/webrtc/v3"
	"go.uber.org/zap"
	"golang.org/x/exp/slices"
)

// WebRTCDialerOptions ...
type WebRTCDialerOptions struct {
	ICEServers []string
	PortMin    uint16
	PortMax    uint16
	UDPMux     ice.UDPMux
	TCPMux     ice.TCPMux
	HostIP     string
}

func parseIPPort(addr string) (net.IP, int, error) {
	host, port, err := net.SplitHostPort(addr)
	if err != nil {
		return nil, 0, fmt.Errorf("malformed address: %w", err)
	}
	ip := net.ParseIP(host)
	if ip == nil {
		return nil, 0, errors.New("malformed ip")
	}
	p, err := strconv.Atoi(port)
	if err != nil {
		return nil, 0, fmt.Errorf("malformed port: %w", err)
	}
	return ip, p, nil
}

func NewWebRTCUDPMux(address string) (ice.UDPMux, *net.UDPConn, error) {
	ip, port, err := parseIPPort(address)
	if err != nil {
		return nil, nil, err
	}
	lis, err := net.ListenUDP("udp", &net.UDPAddr{
		IP:   ip,
		Port: port,
	})
	if err != nil {
		return nil, nil, fmt.Errorf("opening udp listener: %w", err)
	}
	return webrtc.NewICEUDPMux(nil, lis), lis, nil
}

func NewWebRTCTCPMux(address string, readBufferSize int) (ice.TCPMux, *net.TCPListener, error) {
	ip, port, err := parseIPPort(address)
	if err != nil {
		return nil, nil, err
	}
	lis, err := net.ListenTCP("tcp", &net.TCPAddr{
		IP:   ip,
		Port: port,
	})
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

// NewWebRTCDialer ...
func NewWebRTCDialer(logger *zap.Logger, o *WebRTCDialerOptions) *WebRTCDialer {
	if o == nil {
		o = &WebRTCDialerOptions{}
	}
	return &WebRTCDialer{
		logger:  logger,
		options: o,
	}
}

// WebRTCDialer ...
type WebRTCDialer struct {
	logger  *zap.Logger
	options *WebRTCDialerOptions
}

// Dial ...
func (d WebRTCDialer) Dial(m WebRTCMediator) (Link, error) {
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

	s := webrtc.SettingEngine{}

	networkTypes := []webrtc.NetworkType{webrtc.NetworkTypeUDP4, webrtc.NetworkTypeUDP6}
	if d.options.UDPMux != nil {
		s.SetICEUDPMux(d.options.UDPMux)
	} else {
		s.SetEphemeralUDPPortRange(d.options.PortMin, d.options.PortMax)
	}
	if d.options.TCPMux != nil {
		s.SetICETCPMux(d.options.TCPMux)
		networkTypes = append(networkTypes, webrtc.NetworkTypeTCP4, webrtc.NetworkTypeTCP6)
	}
	s.SetNetworkTypes(networkTypes)

	if d.options.HostIP != "" {
		s.SetNAT1To1IPs([]string{d.options.HostIP}, webrtc.ICECandidateTypeHost)
	}

	s.DetachDataChannels()

	api := webrtc.NewAPI(webrtc.WithSettingEngine(s))

	pc, err := api.NewPeerConnection(config)
	if err != nil {
		return nil, err
	}

	link, err := d.dialWebRTC(m, pc)
	if err != nil {
		pc.Close()
		return nil, err
	}
	return link, nil
}

func (d WebRTCDialer) dialWebRTC(m WebRTCMediator, pc *webrtc.PeerConnection) (Link, error) {
	pc.OnConnectionStateChange(func(s webrtc.PeerConnectionState) {
		d.logger.Debug("connection state changed", zap.String("state", s.String()))
		if s == webrtc.PeerConnectionStateDisconnected || s == webrtc.PeerConnectionStateFailed {
			pc.Close()
		}
	})

	// TODO: close this if there's an error gathering ice candidates
	candidates := make(chan *webrtc.ICECandidate, 64)
	pc.OnICECandidate(func(candidate *webrtc.ICECandidate) {
		candidates <- candidate
		if candidate == nil {
			close(candidates)
		}
	})

	dcReady := make(chan *webrtc.DataChannel, 1)
	pc.OnDataChannel(func(dc *webrtc.DataChannel) {
		d.logger.Debug("DataChannel received")
		dc.OnOpen(func() {
			d.logger.Debug("DataChannel opened")
			dcReady <- dc
		})
	})

	offerBytes, err := m.GetOffer()
	if err != nil {
		return nil, err
	}
	if offerBytes != nil {
		var offer webrtc.SessionDescription
		if err := json.Unmarshal(offerBytes, &offer); err != nil {
			return nil, err
		}

		if err := pc.SetRemoteDescription(offer); err != nil {
			return nil, err
		}

		answer, err := pc.CreateAnswer(nil)
		if err != nil {
			return nil, err
		}

		gatherComplete := webrtc.GatheringCompletePromise(pc)

		if err := pc.SetLocalDescription(answer); err != nil {
			return nil, err
		}

		<-gatherComplete

		answerBytes, err := json.Marshal(pc.LocalDescription())
		if err != nil {
			return nil, err
		}
		if err := m.SendAnswer(answerBytes); err != nil {
			return nil, err
		}
	} else {
		dc, err := pc.CreateDataChannel("data", nil)
		if err != nil {
			return nil, err
		}
		dc.OnError(func(err error) {
			d.logger.Debug("DataChannel error", zap.Error(err))
		})
		dc.OnClose(func() {
			d.logger.Debug("DataChannel closed")
		})
		dc.OnOpen(func() {
			d.logger.Debug("DataChannel opened")
			dcReady <- dc
		})

		offer, err := pc.CreateOffer(nil)
		if err != nil {
			return nil, err
		}

		gatherComplete := webrtc.GatheringCompletePromise(pc)

		if err := pc.SetLocalDescription(offer); err != nil {
			return nil, err
		}

		<-gatherComplete

		offerBytes, err := json.Marshal(pc.LocalDescription())
		if err != nil {
			return nil, err
		}
		if err := m.SendOffer(offerBytes); err != nil {
			return nil, err
		}

		answerBytes, err := m.GetAnswer()
		if err != nil {
			return nil, err
		}
		var answer webrtc.SessionDescription
		if err := json.Unmarshal(answerBytes, &answer); err != nil {
			return nil, err
		}
		if err := pc.SetRemoteDescription(answer); err != nil {
			return nil, err
		}
	}

	select {
	case dc := <-dcReady:
		rwc, err := dc.Detach()
		if err != nil {
			return nil, err
		}

		return newDCLink(rwc), nil
	case <-time.After(time.Second * 10):
		return nil, fmt.Errorf("data channel receive timeout")
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
