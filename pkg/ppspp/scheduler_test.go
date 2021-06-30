package ppspp

import (
	"errors"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
)

type mockPeerThing struct {
	id                   []byte
	addReceivedBytesFunc func(n uint64, t timeutil.Time)
	enqueueFunc          func(w peerWriter)
	enqueueNowFunc       func(w peerWriter)
	pushDataFunc         func(w peerWriter, b binmap.Bin, t timeutil.Time, pri peerPriority)
	pushFrontDataFunc    func(w peerWriter, b binmap.Bin, t timeutil.Time, pri peerPriority)
	removeDataFunc       func(w peerWriter, b binmap.Bin, pri peerPriority)
	closeChannelFunc     func(w peerWriter)
}

func (p *mockPeerThing) ID() []byte {
	return p.id
}
func (p *mockPeerThing) AddReceivedBytes(n uint64, t timeutil.Time) {
	if p.addReceivedBytesFunc != nil {
		p.addReceivedBytesFunc(n, t)
	}
}
func (p *mockPeerThing) Enqueue(w peerWriter) {
	if p.enqueueFunc != nil {
		p.enqueueFunc(w)
	}
}
func (p *mockPeerThing) EnqueueNow(w peerWriter) {
	if p.enqueueNowFunc != nil {
		p.enqueueNowFunc(w)
	}
}
func (p *mockPeerThing) PushData(w peerWriter, b binmap.Bin, t timeutil.Time, pri peerPriority) {
	if p.pushDataFunc != nil {
		p.pushDataFunc(w, b, t, pri)
	}
}
func (p *mockPeerThing) PushFrontData(w peerWriter, b binmap.Bin, t timeutil.Time, pri peerPriority) {
	if p.pushFrontDataFunc != nil {
		p.pushFrontDataFunc(w, b, t, pri)
	}
}
func (p *mockPeerThing) RemoveData(w peerWriter, b binmap.Bin, pri peerPriority) {
	if p.removeDataFunc != nil {
		p.removeDataFunc(w, b, pri)
	}
}
func (p *mockPeerThing) CloseChannel(w peerWriter) {
	if p.closeChannelFunc != nil {
		p.closeChannelFunc(w)
	}
}

type mockChannelWriterThing struct {
	peerWriterQueueTicket
	cap, size                int
	ResizeFunc               func(n int) error
	LenFunc                  func()
	FlushFunc                func() error
	ResetFunc                func()
	WriteHandshakeFunc       func(m codec.Handshake) error
	WriteAckFunc             func(m codec.Ack) error
	WriteHaveFunc            func(m codec.Have) error
	WriteDataFunc            func(m codec.Data) error
	WriteIntegrityFunc       func(m codec.Integrity) error
	WriteSignedIntegrityFunc func(m codec.SignedIntegrity) error
	WriteRequestFunc         func(m codec.Request) error
	WritePingFunc            func(m codec.Ping) error
	WritePongFunc            func(m codec.Pong) error
	WriteCancelFunc          func(m codec.Cancel) error
	WriteChokeFunc           func(m codec.Choke) error
	WriteUnchokeFunc         func(m codec.Unchoke) error
	WriteStreamRequestFunc   func(m codec.StreamRequest) error
	WriteStreamCancelFunc    func(m codec.StreamCancel) error
	WriteStreamOpenFunc      func(m codec.StreamOpen) error
	WriteStreamCloseFunc     func(m codec.StreamClose) error
}

func (w *mockChannelWriterThing) Resize(n int) error {
	var err error
	if w.ResizeFunc != nil {
		err = w.ResizeFunc(n)
	}
	if w.size > w.cap {
		return errors.New("not enough space")
	}
	w.size = n
	return err
}
func (w *mockChannelWriterThing) Len() int {
	if w.LenFunc != nil {
		w.LenFunc()
	}
	return w.size
}
func (w *mockChannelWriterThing) Flush() error {
	var err error
	if w.FlushFunc != nil {
		err = w.FlushFunc()
	}
	w.size = w.cap
	return err
}
func (w *mockChannelWriterThing) Reset() {
	if w.ResetFunc != nil {
		w.ResetFunc()
	}
}
func (w *mockChannelWriterThing) WriteHandshake(m codec.Handshake) (int, error) {
	var err error
	if w.WriteHandshakeFunc != nil {
		err = w.WriteHandshakeFunc(m)
	}
	return m.ByteLen(), err
}
func (w *mockChannelWriterThing) WriteAck(m codec.Ack) (int, error) {
	var err error
	if w.WriteAckFunc != nil {
		err = w.WriteAckFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockChannelWriterThing) WriteHave(m codec.Have) (int, error) {
	var err error
	if w.WriteHaveFunc != nil {
		err = w.WriteHaveFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockChannelWriterThing) WriteData(m codec.Data) (int, error) {
	var err error
	if w.WriteDataFunc != nil {
		err = w.WriteDataFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockChannelWriterThing) WriteIntegrity(m codec.Integrity) (int, error) {
	var err error
	if w.WriteIntegrityFunc != nil {
		err = w.WriteIntegrityFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockChannelWriterThing) WriteSignedIntegrity(m codec.SignedIntegrity) (int, error) {
	var err error
	if w.WriteSignedIntegrityFunc != nil {
		err = w.WriteSignedIntegrityFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockChannelWriterThing) WriteRequest(m codec.Request) (int, error) {
	var err error
	if w.WriteRequestFunc != nil {
		err = w.WriteRequestFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockChannelWriterThing) WritePing(m codec.Ping) (int, error) {
	var err error
	if w.WritePingFunc != nil {
		err = w.WritePingFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockChannelWriterThing) WritePong(m codec.Pong) (int, error) {
	var err error
	if w.WritePongFunc != nil {
		err = w.WritePongFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockChannelWriterThing) WriteCancel(m codec.Cancel) (int, error) {
	var err error
	if w.WriteCancelFunc != nil {
		err = w.WriteCancelFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockChannelWriterThing) WriteChoke(m codec.Choke) (int, error) {
	var err error
	if w.WriteChokeFunc != nil {
		err = w.WriteChokeFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockChannelWriterThing) WriteUnchoke(m codec.Unchoke) (int, error) {
	var err error
	if w.WriteUnchokeFunc != nil {
		err = w.WriteUnchokeFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockChannelWriterThing) WriteStreamRequest(m codec.StreamRequest) (int, error) {
	var err error
	if w.WriteStreamRequestFunc != nil {
		err = w.WriteStreamRequestFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockChannelWriterThing) WriteStreamCancel(m codec.StreamCancel) (int, error) {
	var err error
	if w.WriteStreamCancelFunc != nil {
		err = w.WriteStreamCancelFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockChannelWriterThing) WriteStreamOpen(m codec.StreamOpen) (int, error) {
	var err error
	if w.WriteStreamOpenFunc != nil {
		err = w.WriteStreamOpenFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockChannelWriterThing) WriteStreamClose(m codec.StreamClose) (int, error) {
	var err error
	if w.WriteStreamCloseFunc != nil {
		err = w.WriteStreamCloseFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
