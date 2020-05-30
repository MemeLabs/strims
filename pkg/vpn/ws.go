package vpn

import "net/url"

// WebSocketAddr ...
type WebSocketAddr string

// Scheme ...
func (w WebSocketAddr) Scheme() string {
	u, err := url.Parse(w.String())
	if err != nil {
		return ""
	}
	return u.Scheme
}

// String ...
func (w WebSocketAddr) String() string {
	return string(w)
}
