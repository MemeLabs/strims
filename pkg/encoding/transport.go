package encoding

import (
	"context"
	"strings"
	"sync"
)

// Transport ...
type Transport interface {
	Write([]byte, TransportConn) error
	Read([]byte) (int, TransportConn, error)
	Listen(context.Context) error
	Close() error
	Status() TransportStatus
	MTU() int
	Dial(TransportURI) (TransportConn, error)
	Scheme() string
}

// TransportConn ...
type TransportConn interface {
	URI() TransportURI
	Transport() Transport
	Write([]byte) error
	Close() error
}

// TransportURI ...
type TransportURI string

// Scheme ...
func (t TransportURI) Scheme() (s string) {
	if i := strings.Index(string(t), "://"); i != -1 {
		s = string(t)[:i+3]
	}
	return
}

// TransportStatus ...
type TransportStatus int

// statuses
const (
	StatusClosed TransportStatus = iota
	StatusListening
	StatusError TransportStatus = -1
)

type transportState struct {
	sync.Mutex
	status TransportStatus
}

func (t *transportState) setStatus(s TransportStatus) (prev TransportStatus) {
	t.Lock()
	defer t.Unlock()

	prev = t.status
	t.status = s
	return
}

func (t *transportState) Status() TransportStatus {
	t.Lock()
	defer t.Unlock()
	return t.status
}
