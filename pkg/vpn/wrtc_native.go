// +build !js

package vpn

import (
	"encoding/json"
	"fmt"
	"io"
	"log"
	"math/rand"
	"time"

	"github.com/pion/webrtc/v2"
)

// WebRTCDialer ...
type WebRTCDialer struct{}

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
	pc.OnICEConnectionStateChange(func(s webrtc.ICEConnectionState) {
		log.Println("connection state changed to", s.String())
		if s == webrtc.ICEConnectionStateClosed {
			// a.handleClose()
		}
	})

	// TODO: close this if there's an error gathering ice candidates
	candidates := make(chan *webrtc.ICECandidate, 32)
	pc.OnICECandidate(func(candidate *webrtc.ICECandidate) {
		candidates <- candidate
		if candidate == nil {
			close(candidates)
		}
	})

	try := rand.Int()

	dcReady := make(chan *webrtc.DataChannel, 1)
	pc.OnDataChannel(func(dc *webrtc.DataChannel) {
		log.Println("got dc from peer --------------------------", try)
		dc.OnOpen(func() {
			log.Println("dc onopen fired --------------------------------", try)
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
		log.Println("creating dc --------------------------------", try)
		dc, err := pc.CreateDataChannel("data", nil)
		if err != nil {
			return nil, err
		}
		// dc.OnError(func(err error) {
		// 	log.Println("there was an error", err, try)
		// })
		dc.OnClose(func() {
			log.Println("that shit was closed", try)
		})
		dc.OnOpen(func() {
			log.Println("dc onopen fired --------------------------------", try)
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
					log.Println(err)
					continue
				}
			}
			if err := m.SendICECandidate(b); err != nil {
				log.Println(err)
			}
		}
	}()

	go func() {
		for b := range m.GetICECandidates() {
			var c webrtc.ICECandidateInit
			if err := json.Unmarshal(b, &c); err != nil {
				log.Println(err)
				continue
			}
			if err := pc.AddICECandidate(c); err != nil {
				log.Println(err)
			}
		}
	}()

	select {
	case dc := <-dcReady:
		rwc, err := dc.Detach()
		if err != nil {
			return nil, err
		}
		return dcLink{rwc}, nil
	case <-time.After(time.Second * 10):
		return nil, fmt.Errorf("data channel receive timeout %d", try)
	}
}

type dcLink struct {
	io.ReadWriteCloser
}

func (l dcLink) MTU() int {
	return 16 * 1024
}

func newBool(v bool) *bool {
	return &v
}
func newUint16(v uint16) *uint16 {
	return &v
}
