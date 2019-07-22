package encoding

import (
	"log"
	"net"
)

type UDPConn struct {
	t *UDPTransport
	a net.Addr
}

func (c UDPConn) addressInterface() {}

func (c UDPConn) String() string {
	return c.a.String()
}

func (c UDPConn) Write(b []byte) (err error) {
	return c.t.Write(b, c)
}

func NewUDPTransport() *UDPTransport {
	return &UDPTransport{}
}

type UDPTransport struct {
	transportState
	Address string
	conn    net.PacketConn
}

func (t *UDPTransport) MTU() int {
	// TODO: should this be discovered per remote address?
	return 1500
}

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

func (t *UDPTransport) Close() error {
	if t.setStatus(StatusClosed) != StatusListening {
		return nil
	}
	return t.conn.Close()
}

func (t *UDPTransport) Read() (b []byte, a TransportConn, err error) {
	var n int
	ua := UDPConn{t: t}
	b = make([]byte, 1500)
	n, ua.a, err = t.conn.ReadFrom(b)
	if err != nil {
		return
	}
	return b[:n], ua, nil
}

func (t *UDPTransport) Write(b []byte, a TransportConn) (err error) {
	_, err = t.conn.WriteTo(b, a.(UDPConn).a)
	return
}
