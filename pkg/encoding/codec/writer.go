package codec

import (
	"io"
)

func NewWriter(w io.Writer, size int) Writer {
	return Writer{
		w:    w,
		size: size,
		buf:  make([]byte, size),
	}
}

type Writer struct {
	w    io.Writer
	size int
	buf  []byte
	off  int
}

type flusher interface {
	Flush() error
}

func (w *Writer) Flush() error {
	if w.off == 0 {
		return nil
	}

	if _, err := w.w.Write(w.buf[:w.off]); err != nil {
		return err
	}

	w.off = 0

	if f, ok := w.w.(flusher); ok {
		return f.Flush()
	}

	return nil
}

func (w *Writer) Write(m Message) (int, error) {
	n := m.ByteLen() + 1
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

func (w *Writer) WriteHandshake(m Handshake) (int, error) {
	n := m.ByteLen() + 1
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

func (w *Writer) WriteAck(m Ack) (int, error) {
	n := m.ByteLen() + 1
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

func (w *Writer) WriteHave(m Have) (int, error) {
	n := m.ByteLen() + 1
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

func (w *Writer) WriteData(m Data) (int, error) {
	n := m.ByteLen() + 1
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

func (w *Writer) WriteRequest(m Request) (int, error) {
	n := m.ByteLen() + 1
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

func (w *Writer) WritePing(m Ping) (int, error) {
	n := m.ByteLen() + 1
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

func (w *Writer) WritePong(m Pong) (int, error) {
	n := m.ByteLen() + 1
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

func (w *Writer) WriteCancel(m Cancel) (int, error) {
	n := m.ByteLen() + 1
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

func (w *Writer) ensureSpace(n int) error {
	if w.off+n > w.size {
		if err := w.Flush(); err != nil {
			return err
		}
	}
	return nil
}
