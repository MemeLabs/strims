package encoding

import (
	"context"
	"errors"
	"log"
	"net"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/pion/dtls"
)

// DTLSScheme TransportURI scheme
const DTLSScheme = "dtls://"

// DTLSConn ...
type DTLSConn struct {
	t *DTLSTransport
	a net.Addr
	u TransportURI
	c net.Conn
}

// Transport ...
func (c DTLSConn) Transport() Transport {
	return c.t
}

// URI ...
func (c DTLSConn) URI() TransportURI {
	return c.u
}

// Write ...
func (c DTLSConn) Write(b []byte) (err error) {
	return c.t.Write(b, c)
}

// Close ...
func (c DTLSConn) Close() error {
	return nil
}

// NewDTLSTransport ...
func NewDTLSTransport() *DTLSTransport {
	return &DTLSTransport{}
}

// DTLSTransport ...
type DTLSTransport struct {
	transportState
	Address string
	conn    *dtls.Listener
	read    chan *dtlsRead
}

type dtlsRead struct {
	d []byte
	c DTLSConn
}

// MTU ...
func (t *DTLSTransport) MTU() int {
	return 1200
}

// Listen ...
func (t *DTLSTransport) Listen(ctx context.Context) (err error) {
	// debug.Green("creating read")
	t.read = make(chan *dtlsRead, 16)

	addr, err := parseAddr(t.Address)
	if err != nil {
		return
	}
	log.Println("listening", addr)

	// Generate a certificate and private key to secure the connection
	certificate, key, err := dtls.GenerateSelfSigned()
	if err != nil {
		return err
	}

	// Prepare the configuration of the DTLS connection
	config := &dtls.Config{
		Certificate:          certificate,
		PrivateKey:           key,
		InsecureSkipVerify:   true,
		ExtendedMasterSecret: dtls.RequireExtendedMasterSecret,
		ConnectTimeout:       dtls.ConnectTimeoutOption(30 * time.Second),
	}

	// Connect to a DTLS server
	t.conn, err = dtls.Listen("udp", addr, config)

	go t.handleConns()

	log.Println("dtls transport listening at", t.Address)
	t.setStatus(StatusListening)

	return
}

var readBufs = sync.Pool{
	New: func() interface{} {
		return make([]byte, 1200)
	},
}

func (t *DTLSTransport) handleConns() error {
	defer func() {
		// debug.Cyan("closing t.read")
		t.read <- nil
	}()

	for {
		c, err := t.conn.Accept()
		if err != nil {
			// debug.Red("error in handleConns", err)
			return err
		}

		cc := DTLSConn{
			t: t,
			u: TransportURI(DTLSScheme + c.RemoteAddr().String()),
			c: c,
		}

		go func(cc DTLSConn) {
			for {
				b := readBufs.Get().([]byte)
				n, err := c.Read(b)
				if err != nil {
					// debug.Red(err)
					return
				}
				// debug.Green("read", n, b[:n])
				t.read <- &dtlsRead{
					d: b[:n],
					c: cc,
				}
			}
		}(cc)
	}
}

// Close ...
func (t *DTLSTransport) Close() error {
	if t.setStatus(StatusClosed) != StatusListening {
		return nil
	}
	return t.conn.Close(30 * time.Second)
}

// Read ...
func (t *DTLSTransport) Read(b []byte) (n int, a TransportConn, err error) {
	// debug.Blue("trying to read from dtls transport", t.read)
	r := <-t.read
	if r == nil {
		// debug.Red("returned transport closed error from read")
		return 0, nil, errors.New("transport closed")
	}
	n = copy(b, r.d)
	// debug.Blue("read some bytes", n, b[:n])
	readBufs.Put(r.d[:1200])
	return n, r.c, nil
}

// Write ...
func (t *DTLSTransport) Write(b []byte, a TransportConn) (err error) {
	_, err = a.(DTLSConn).c.Write(b)
	return
}

// Dial ...
func (t *DTLSTransport) Dial(uri TransportURI) (tc TransportConn, err error) {
	addr, err := parseAddr(strings.TrimPrefix(string(uri), DTLSScheme))
	if err != nil {
		return
	}

	// Generate a certificate and private key to secure the connection
	certificate, key, err := dtls.GenerateSelfSigned()
	if err != nil {
		return
	}

	//
	// Everything below is the pion-DTLS API! Thanks for using it ❤️.
	//

	// Prepare the configuration of the DTLS connection
	config := &dtls.Config{
		Certificate:          certificate,
		PrivateKey:           key,
		InsecureSkipVerify:   true,
		ExtendedMasterSecret: dtls.RequireExtendedMasterSecret,
		ConnectTimeout:       dtls.ConnectTimeoutOption(30 * time.Second),
	}

	// debug.Yellow("dialing", addr)

	// Connect to a DTLS server
	c, err := dtls.Dial("udp", addr, config)
	if err != nil {
		return
	}

	tc = DTLSConn{
		t: t,
		u: uri,
		c: c,
	}

	go func(tc DTLSConn) {
		for {
			b := readBufs.Get().([]byte)
			n, err := c.Read(b)
			if err != nil {
				return
			}
			// debug.Yellow("read", n)
			t.read <- &dtlsRead{
				d: b[:n],
				c: tc,
			}
		}
	}(tc.(DTLSConn))

	return
}

// Scheme ...
func (t *DTLSTransport) Scheme() string {
	return DTLSScheme
}

func parseAddr(s string) (*net.UDPAddr, error) {
	host, port, err := net.SplitHostPort(s)
	if err != nil {
		return nil, err
	}

	addr := &net.UDPAddr{IP: net.ParseIP(host)}
	addr.Port, err = strconv.Atoi(port)
	if err != nil {
		return nil, err
	}
	return addr, nil
}
