package vnic

// WebRTCScheme webrtc url scheme
const WebRTCScheme = "webrtc"

// NewWebRTCInterface ...
func NewWebRTCInterface(d *WebRTCDialer) *WebRTCInterface {
	return &WebRTCInterface{d}
}

// WebRTCInterface ...
type WebRTCInterface struct {
	Dialer *WebRTCDialer
}

// ValidScheme ...
func (w *WebRTCInterface) ValidScheme(scheme string) bool {
	return scheme == WebRTCScheme
}

// Dial ...
func (w *WebRTCInterface) Dial(h *Host, addr InterfaceAddr) error {
	c, err := w.Dialer.Dial(addr.(WebRTCMediator))
	if err != nil {
		return err
	}
	h.AddLink(c)
	return nil
}

// WebRTCMediator ...
type WebRTCMediator interface {
	Scheme() string
	GetOffer() ([]byte, error)
	GetAnswer() ([]byte, error)
	GetICECandidates() <-chan []byte
	SendOffer([]byte) error
	SendAnswer([]byte) error
	SendICECandidate([]byte) error
}
