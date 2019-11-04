package encoding

import (
	"context"
	"log"
	"net"
	"strings"
)

// UDPScheme TransportURI scheme
const UDPScheme = "udp://"

// UDPConn ...
type UDPConn struct {
	t *UDPTransport
	a net.Addr
}

// Transport ...
func (c UDPConn) Transport() Transport {
	return c.t
}

// URI ...
func (c UDPConn) URI() TransportURI {
	return TransportURI(UDPScheme + c.a.String())
}

// Write ...
func (c UDPConn) Write(b []byte) (err error) {
	return c.t.Write(b, c)
}

// Close ...
func (c UDPConn) Close() error {
	return nil
}

// NewUDPTransport ...
func NewUDPTransport() *UDPTransport {
	return &UDPTransport{}
}

// UDPTransport ...
type UDPTransport struct {
	transportState
	Address string
	conn    net.PacketConn
}

// MTU ...
func (t *UDPTransport) MTU() int {
	return 1200
}

// Listen ...
func (t *UDPTransport) Listen(ctx context.Context) (err error) {
	t.conn, err = net.ListenPacket("udp", t.Address)
	if err != nil {
		t.setStatus(StatusError)
		return
	}

	log.Println("udp transport listening at", t.Address)
	t.setStatus(StatusListening)
	return
}

// Close ...
func (t *UDPTransport) Close() error {
	if t.setStatus(StatusClosed) != StatusListening {
		return nil
	}
	return t.conn.Close()
}

// Read ...
func (t *UDPTransport) Read(b []byte) (n int, a TransportConn, err error) {
	ua := UDPConn{t: t}
	n, ua.a, err = t.conn.ReadFrom(b)
	if err != nil {
		return
	}
	return n, ua, nil
}

// Write ...
func (t *UDPTransport) Write(b []byte, a TransportConn) (err error) {
	_, err = t.conn.WriteTo(b, a.(UDPConn).a)
	return
}

// Dial ...
func (t *UDPTransport) Dial(uri TransportURI) (tc TransportConn, err error) {
	as := strings.TrimPrefix(string(uri), UDPScheme)
	a, err := net.ResolveUDPAddr("udp", as)
	if err != nil {
		return
	}

	tc = UDPConn{
		t: t,
		a: a,
	}
	return
}

// Scheme ...
func (t *UDPTransport) Scheme() string {
	return UDPScheme
}
