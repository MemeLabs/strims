package encoding

import (
	"log"
	"net"
)

// UDPConn ...
type UDPConn struct {
	t *UDPTransport
	a net.Addr
}

// addressInterface ...
func (c UDPConn) addressInterface() {}

// Transport ...
func (c UDPConn) Transport() Transport {
	return c.t
}

// String ...
func (c UDPConn) String() string {
	return c.a.String()
}

// Write ...
func (c UDPConn) Write(b []byte) (err error) {
	return c.t.Write(b, c)
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
func (t *UDPTransport) Listen() (err error) {
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
