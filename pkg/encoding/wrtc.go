package encoding

import (
	"context"
	"errors"
	"strings"
)

// WRTCScheme TransportURI scheme
const WRTCScheme = "wrtc://"

type wrtcConn interface {
	isWRTCConn()
}

// WRTCAdapter ...
type WRTCAdapter interface {
	write([]byte, wrtcConn) error
	read([]byte) (int, wrtcConn, error)
	closeConn(wrtcConn) error
	listen(context.Context) error
	dial(TransportURI) (wrtcConn, error)
	close() error
}

// WRTCConn ...
type WRTCConn struct {
	c   wrtcConn
	t   *WRTCTransport
	uri TransportURI
}

// Transport ...
func (c WRTCConn) Transport() Transport {
	return c.t
}

// URI ...
func (c WRTCConn) URI() TransportURI {
	// encode rendezvous

	// connect to signal server - get assigned unique id
	// maintain websocket connection to rendezvous server to receive connection requests
	// share uri as wrtc://rendezvous.server/myid

	// allow negotiating connections through any mutual peer?

	return c.uri
}

// Write ...
func (c WRTCConn) Write(b []byte) (err error) {
	return c.t.Write(b, c)
}

// Close ...
func (c WRTCConn) Close() error {
	return c.t.Adapter.closeConn(c.c)
}

// WRTCTransport ...
type WRTCTransport struct {
	transportState
	Adapter WRTCAdapter
}

// MTU ...
func (t *WRTCTransport) MTU() int {
	return 32000
}

// Listen ...
func (t *WRTCTransport) Listen(ctx context.Context) (err error) {
	t.Adapter.listen(ctx)
	t.setStatus(StatusListening)
	return
}

// Close ...
func (t *WRTCTransport) Close() error {
	if t.setStatus(StatusClosed) != StatusListening {
		return nil
	}
	return t.Adapter.close()
}

// Read ...
func (t *WRTCTransport) Read(b []byte) (n int, c TransportConn, err error) {
	c = &WRTCConn{
		t: t,
	}
	n, c.(*WRTCConn).c, err = t.Adapter.read(b)
	return
}

// Write ...
func (t *WRTCTransport) Write(b []byte, a TransportConn) (err error) {
	return t.Adapter.write(b, a.(WRTCConn).c)
}

// Dial ...
func (t *WRTCTransport) Dial(uri TransportURI) (tc TransportConn, err error) {
	c, err := t.Adapter.dial(uri)
	if err != nil {
		return
	}

	tc = WRTCConn{
		c:   c,
		t:   t,
		uri: uri,
	}
	return
}

// Scheme ...
func (t *WRTCTransport) Scheme() string {
	return WRTCScheme
}

func resolveThing(uri TransportURI) (t *thing, err error) {
	parts := strings.Split(strings.TrimPrefix(string(uri), WRTCScheme), "/")
	if len(parts) != 3 {
		err = errors.New("invalid thing format")
		return
	}

	t = &thing{
		sp: parts[0],
		sa: parts[1],
		id: parts[2],
	}
	return
}

type thing struct {
	sp string
	sa string
	id string
}
