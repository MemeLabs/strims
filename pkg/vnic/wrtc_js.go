// +build js,wasm

package vnic

import (
	"encoding/json"
	"syscall/js"

	"github.com/MemeLabs/go-ppspp/pkg/wasmio"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
)

// NewWebRTCDialer ...
func NewWebRTCDialer(logger *zap.Logger, bridge js.Value) *WebRTCDialer {
	return &WebRTCDialer{logger, bridge}
}

// WebRTCDialer ...
type WebRTCDialer struct {
	logger *zap.Logger
	bridge js.Value
}

// Dial ...
func (d *WebRTCDialer) Dial(m WebRTCMediator) (Link, error) {
	pc := wasmio.NewWebRTCProxy(d.bridge)

	link, err := d.dial(m, pc)
	if err != nil {
		pc.Close()
		return nil, err
	}
	return link, nil
}

func (d *WebRTCDialer) dial(m WebRTCMediator, pc *wasmio.WebRTCProxy) (Link, error) {
	offerBytes, err := m.GetOffer()
	if err != nil {
		return nil, err
	}
	if offerBytes != nil {
		if err := d.sendAnswer(m, pc, offerBytes); err != nil {
			return nil, err
		}
	} else {
		if err := d.sendOffer(m, pc); err != nil {
			return nil, err
		}
	}

	var g errgroup.Group
	g.Go(func() error { return d.recieveICECandidates(m, pc) })
	g.Go(func() error { return d.sendICECandidates(m, pc) })
	if err := g.Wait(); err != nil {
		return nil, err
	}

	cid, err := pc.DataChannelID("data")
	if err != nil {
		return nil, err
	}
	return wasmio.NewDataChannelProxy(d.bridge, cid)
}

func (d *WebRTCDialer) recieveICECandidates(m WebRTCMediator, pc *wasmio.WebRTCProxy) error {
	for b := range m.GetICECandidates() {
		var c wasmio.ICECandidateInit
		if err := json.Unmarshal(b, &c); err != nil {
			return err
		}
		pc.AddICECandidate(&c)
	}
	return nil
}

func (d *WebRTCDialer) sendICECandidates(m WebRTCMediator, pc *wasmio.WebRTCProxy) error {
	for c := range pc.ICECandidates() {
		var b []byte
		var err error
		if c != nil {
			b, err = json.Marshal(c)
			if err != nil {
				return err
			}
		}
		if err := m.SendICECandidate(b); err != nil {
			return err
		}
	}
	return nil
}

func (d *WebRTCDialer) sendAnswer(m WebRTCMediator, pc *wasmio.WebRTCProxy, offerBytes []byte) error {
	var offer wasmio.RTCSessionDescription
	if err := json.Unmarshal(offerBytes, &offer); err != nil {
		return err
	}
	pc.SetRemoteDescription(&offer)

	answer, err := pc.CreateAnswer()
	if err != nil {
		return err
	}
	pc.SetLocalDescription(answer)

	answerBytes, err := json.Marshal(answer)
	if err != nil {
		return err
	}
	if err := m.SendAnswer(answerBytes); err != nil {
		return err
	}
	return nil
}

func (d *WebRTCDialer) sendOffer(m WebRTCMediator, pc *wasmio.WebRTCProxy) error {
	pc.CreateDataChannel("data")

	offer, err := pc.CreateOffer()
	if err != nil {
		return err
	}
	pc.SetLocalDescription(offer)

	offerBytes, err := json.Marshal(offer)
	if err != nil {
		return err
	}
	if err := m.SendOffer(offerBytes); err != nil {
		return err
	}

	answerBytes, err := m.GetAnswer()
	if err != nil {
		return err
	}
	var answer wasmio.RTCSessionDescription
	if err := json.Unmarshal(answerBytes, &answer); err != nil {
		return err
	}
	pc.SetRemoteDescription(&answer)
	return nil
}
