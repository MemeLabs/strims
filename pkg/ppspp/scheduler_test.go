// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ppspp

import (
	"github.com/MemeLabs/strims/pkg/binmap"
	"github.com/MemeLabs/strims/pkg/ppspp/codec"
	"github.com/MemeLabs/strims/pkg/timeutil"
)

type mockPeerTaskQueue struct {
	id                   []byte
	addReceivedBytesFunc func(n uint64, t timeutil.Time)
	enqueueFunc          func(w peerTaskRunner)
	enqueueNowFunc       func(w peerTaskRunner)
	pushDataFunc         func(w peerTaskRunner, b binmap.Bin, t timeutil.Time, pri peerPriority)
	pushFrontDataFunc    func(w peerTaskRunner, b binmap.Bin, t timeutil.Time, pri peerPriority)
	removeDataFunc       func(w peerTaskRunner, b binmap.Bin, pri peerPriority)
	removeRunnerFunc     func(w peerTaskRunner)
}

func (p *mockPeerTaskQueue) ID() []byte {
	return p.id
}
func (p *mockPeerTaskQueue) AddReceivedBytes(n uint64, t timeutil.Time) {
	if p.addReceivedBytesFunc != nil {
		p.addReceivedBytesFunc(n, t)
	}
}
func (p *mockPeerTaskQueue) Enqueue(w peerTaskRunner) {
	if p.enqueueFunc != nil {
		p.enqueueFunc(w)
	}
}
func (p *mockPeerTaskQueue) EnqueueNow(w peerTaskRunner) {
	if p.enqueueNowFunc != nil {
		p.enqueueNowFunc(w)
	}
}
func (p *mockPeerTaskQueue) PushData(w peerTaskRunner, b binmap.Bin, t timeutil.Time, pri peerPriority) {
	if p.pushDataFunc != nil {
		p.pushDataFunc(w, b, t, pri)
	}
}
func (p *mockPeerTaskQueue) PushFrontData(w peerTaskRunner, b binmap.Bin, t timeutil.Time, pri peerPriority) {
	if p.pushFrontDataFunc != nil {
		p.pushFrontDataFunc(w, b, t, pri)
	}
}
func (p *mockPeerTaskQueue) RemoveData(w peerTaskRunner, b binmap.Bin, pri peerPriority) {
	if p.removeDataFunc != nil {
		p.removeDataFunc(w, b, pri)
	}
}
func (p *mockPeerTaskQueue) RemoveRunner(w peerTaskRunner) {
	if p.removeRunnerFunc != nil {
		p.removeRunnerFunc(w)
	}
}

type mockCodecMessageWriter struct {
	peerTaskRunnerQueueTicket
	cap, size                int
	LenFunc                  func() int
	AvailableFunc            func() int
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

func (w *mockCodecMessageWriter) Len() int {
	if w.LenFunc != nil {
		w.LenFunc()
	}
	return w.size
}
func (w *mockCodecMessageWriter) Available() int {
	if w.AvailableFunc != nil {
		w.AvailableFunc()
	}
	return w.size
}
func (w *mockCodecMessageWriter) Flush() error {
	var err error
	if w.FlushFunc != nil {
		err = w.FlushFunc()
	}
	w.size = w.cap
	return err
}
func (w *mockCodecMessageWriter) Reset() {
	if w.ResetFunc != nil {
		w.ResetFunc()
	}
}
func (w *mockCodecMessageWriter) WriteHandshake(m codec.Handshake) (int, error) {
	var err error
	if w.WriteHandshakeFunc != nil {
		err = w.WriteHandshakeFunc(m)
	}
	return m.ByteLen(), err
}
func (w *mockCodecMessageWriter) WriteAck(m codec.Ack) (int, error) {
	var err error
	if w.WriteAckFunc != nil {
		err = w.WriteAckFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockCodecMessageWriter) WriteHave(m codec.Have) (int, error) {
	var err error
	if w.WriteHaveFunc != nil {
		err = w.WriteHaveFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockCodecMessageWriter) WriteData(m codec.Data) (int, error) {
	var err error
	if w.WriteDataFunc != nil {
		err = w.WriteDataFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockCodecMessageWriter) WriteIntegrity(m codec.Integrity) (int, error) {
	var err error
	if w.WriteIntegrityFunc != nil {
		err = w.WriteIntegrityFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockCodecMessageWriter) WriteSignedIntegrity(m codec.SignedIntegrity) (int, error) {
	var err error
	if w.WriteSignedIntegrityFunc != nil {
		err = w.WriteSignedIntegrityFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockCodecMessageWriter) WriteRequest(m codec.Request) (int, error) {
	var err error
	if w.WriteRequestFunc != nil {
		err = w.WriteRequestFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockCodecMessageWriter) WritePing(m codec.Ping) (int, error) {
	var err error
	if w.WritePingFunc != nil {
		err = w.WritePingFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockCodecMessageWriter) WritePong(m codec.Pong) (int, error) {
	var err error
	if w.WritePongFunc != nil {
		err = w.WritePongFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockCodecMessageWriter) WriteCancel(m codec.Cancel) (int, error) {
	var err error
	if w.WriteCancelFunc != nil {
		err = w.WriteCancelFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockCodecMessageWriter) WriteChoke(m codec.Choke) (int, error) {
	var err error
	if w.WriteChokeFunc != nil {
		err = w.WriteChokeFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockCodecMessageWriter) WriteUnchoke(m codec.Unchoke) (int, error) {
	var err error
	if w.WriteUnchokeFunc != nil {
		err = w.WriteUnchokeFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockCodecMessageWriter) WriteStreamRequest(m codec.StreamRequest) (int, error) {
	var err error
	if w.WriteStreamRequestFunc != nil {
		err = w.WriteStreamRequestFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockCodecMessageWriter) WriteStreamCancel(m codec.StreamCancel) (int, error) {
	var err error
	if w.WriteStreamCancelFunc != nil {
		err = w.WriteStreamCancelFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockCodecMessageWriter) WriteStreamOpen(m codec.StreamOpen) (int, error) {
	var err error
	if w.WriteStreamOpenFunc != nil {
		err = w.WriteStreamOpenFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
func (w *mockCodecMessageWriter) WriteStreamClose(m codec.StreamClose) (int, error) {
	var err error
	if w.WriteStreamCloseFunc != nil {
		err = w.WriteStreamCloseFunc(m)
	}
	w.size -= m.ByteLen()
	return m.ByteLen(), err
}
