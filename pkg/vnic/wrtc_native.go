//go:build !js

package vnic

import (
	"bufio"
	"encoding/json"
	"fmt"
	"io"
	"time"

	"github.com/pion/webrtc/v3"
	"go.uber.org/zap"
)

// WebRTCDialerOptions ...
type WebRTCDialerOptions struct {
	PortMin uint16
	PortMax uint16
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
	// TODO: load this from app config
	config := webrtc.Configuration{
		ICEServers: []webrtc.ICEServer{
			{
				URLs: []string{
					"stun:stun.l.google.com:19302",
					"stun:stun1.l.google.com:19302",
					"stun:stun2.l.google.com:19302",
					"stun:stun3.l.google.com:19302",
					"stun:stun4.l.google.com:19302",
				},
			},
		},
	}

	s := webrtc.SettingEngine{}
	s.SetEphemeralUDPPortRange(d.options.PortMin, d.options.PortMax)
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
		if err := pc.SetLocalDescription(answer); err != nil {
			return nil, err
		}

		answerBytes, err := json.Marshal(answer)
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
		if err := pc.SetLocalDescription(offer); err != nil {
			return nil, err
		}

		offerBytes, err := json.Marshal(offer)
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

	go func() {
		for candidate := range candidates {
			var b []byte
			if candidate != nil {
				b, err = json.Marshal(candidate.ToJSON())
				if err != nil {
					d.logger.Debug("candidate json marshal failed", zap.Error(err))
					continue
				}
			}
			if err := m.SendICECandidate(b); err != nil {
				d.logger.Debug("sending ice candidate failed", zap.Error(err))
			}
		}
	}()

	go func() {
		for b := range m.GetICECandidates() {
			var c webrtc.ICECandidateInit
			if err := json.Unmarshal(b, &c); err != nil {
				d.logger.Debug("getting ice candidate failed", zap.Error(err))
				continue
			}
			if err := pc.AddICECandidate(c); err != nil {
				d.logger.Debug("adding ice candidate failed", zap.Error(err))
			}
		}
	}()

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

func newDCLink(rwc io.ReadWriteCloser) *dcLink {
	return &dcLink{
		Reader:      bufio.NewReaderSize(rwc, 16*1024),
		WriteCloser: rwc,
	}
}

type dcLink struct {
	io.Reader
	io.WriteCloser
}

func (l dcLink) MTU() int {
	return 16 * 1024
}
