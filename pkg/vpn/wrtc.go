package vpn

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
	return scheme == "webrtc"
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
