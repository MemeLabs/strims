package encoding

import "sync"

type Transport interface {
	Write([]byte, TransportConn) error
	Read() ([]byte, TransportConn, error)
	Listen() error
	Close() error
	Status() TransportStatus
	MTU() int
}

type TransportConn interface {
	String() string
	addressInterface()
}

type TransportStatus int

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
