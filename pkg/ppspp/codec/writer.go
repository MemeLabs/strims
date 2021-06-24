package codec

import (
	"errors"
	"io"
)

var (
	ErrTooSmall       = errors.New("new size cannot be smaller than buffered messag length")
	ErrBufferTooSmall = errors.New("new size cannot be larger than write buffer")
	ErrNotEnoughSpace = errors.New("write buffer has insufficient space for message")
)

const MessageTypeLen = 1

// NewWriter ...
func NewWriter(w io.Writer, size int) Writer {
	return Writer{
		w:   w,
		buf: make([]byte, size),
	}
}

// Writer ...
type Writer struct {
	w   io.Writer
	off int
	buf []byte
}

// ensureSpace ...
func (w *Writer) ensureSpace(n int) error {
	if w.off+n > len(w.buf) {
		return ErrNotEnoughSpace
	}
	return nil
}

// Dirty ...
func (w *Writer) Dirty() bool {
	return w.off != 0
}

// Len ...
func (w *Writer) Len() int {
	return w.off
}

func (w *Writer) Resize(n int) error {
	if n < w.off {
		return ErrTooSmall
	}
	if n > cap(w.buf) {
		return ErrBufferTooSmall
	}
	w.buf = w.buf[:n]
	return nil
}

func (w *Writer) Reset() {
	w.off = 0
}

// Flush ...
func (w *Writer) Flush() error {
	if !w.Dirty() {
		return nil
	}

	if _, err := w.w.Write(w.buf[:w.off]); err != nil {
		return err
	}

	w.off = 0

	return nil
}

// Write ...
func (w *Writer) Write(m Message) (int, error) {
	n := m.ByteLen() + MessageTypeLen
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

// WriteHandshake ...
func (w *Writer) WriteHandshake(m Handshake) (int, error) {
	n := m.ByteLen() + MessageTypeLen
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

// WriteAck ...
func (w *Writer) WriteAck(m Ack) (int, error) {
	n := m.ByteLen() + MessageTypeLen
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

// WriteHave ...
func (w *Writer) WriteHave(m Have) (int, error) {
	n := m.ByteLen() + MessageTypeLen
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

// WriteData ...
func (w *Writer) WriteData(m Data) (int, error) {
	n := m.ByteLen() + MessageTypeLen
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

// WriteIntegrity ...
func (w *Writer) WriteIntegrity(m Integrity) (int, error) {
	n := m.ByteLen() + MessageTypeLen
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

// WriteSignedIntegrity ...
func (w *Writer) WriteSignedIntegrity(m SignedIntegrity) (int, error) {
	n := m.ByteLen() + MessageTypeLen
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

// WriteRequest ...
func (w *Writer) WriteRequest(m Request) (int, error) {
	n := m.ByteLen() + MessageTypeLen
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

// WritePing ...
func (w *Writer) WritePing(m Ping) (int, error) {
	n := m.ByteLen() + MessageTypeLen
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

// WritePong ...
func (w *Writer) WritePong(m Pong) (int, error) {
	n := m.ByteLen() + MessageTypeLen
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

// WriteCancel ...
func (w *Writer) WriteCancel(m Cancel) (int, error) {
	n := m.ByteLen() + MessageTypeLen
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

// WriteChoke ...
func (w *Writer) WriteChoke(m Choke) (int, error) {
	n := m.ByteLen() + MessageTypeLen
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

// WriteUnchoke ...
func (w *Writer) WriteUnchoke(m Unchoke) (int, error) {
	n := m.ByteLen() + MessageTypeLen
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

// WriteStreamRequest ...
func (w *Writer) WriteStreamRequest(m StreamRequest) (int, error) {
	n := m.ByteLen() + MessageTypeLen
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

// WriteStreamCancel ...
func (w *Writer) WriteStreamCancel(m StreamCancel) (int, error) {
	n := m.ByteLen() + MessageTypeLen
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

// WriteStreamOpen ...
func (w *Writer) WriteStreamOpen(m StreamOpen) (int, error) {
	n := m.ByteLen() + MessageTypeLen
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

// WriteStreamClose ...
func (w *Writer) WriteStreamClose(m StreamClose) (int, error) {
	n := m.ByteLen() + MessageTypeLen
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}
