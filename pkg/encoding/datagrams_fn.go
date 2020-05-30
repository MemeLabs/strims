package encoding

import "io"

// TODO: maybe escape analysis will work for generics...

func newDatagramWriter(w io.Writer, size int) *datagramWriter {
	return &datagramWriter{
		w:    w,
		size: size,
		buf:  make([]byte, size),
	}
}

type datagramWriter struct {
	w    io.Writer
	size int
	buf  []byte
	off  int
}

type flusher interface {
	Flush() error
}

func (w *datagramWriter) Flush() error {
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

func (w *datagramWriter) Write(m Message) (int, error) {
	n := m.ByteLen() + 1
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

func (w *datagramWriter) WriteAck(m Ack) (int, error) {
	n := m.ByteLen() + 1
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

func (w *datagramWriter) WriteHave(m Have) (int, error) {
	n := m.ByteLen() + 1
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

func (w *datagramWriter) WriteData(m Data) (int, error) {
	n := m.ByteLen() + 1
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

func (w *datagramWriter) WriteRequest(m Request) (int, error) {
	n := m.ByteLen() + 1
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

func (w *datagramWriter) WritePing(m Ping) (int, error) {
	n := m.ByteLen() + 1
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

func (w *datagramWriter) WriteCancel(m Cancel) (int, error) {
	n := m.ByteLen() + 1
	if err := w.ensureSpace(n); err != nil {
		return 0, err
	}

	w.buf[w.off] = byte(m.Type())
	w.off++

	w.off += m.Marshal(w.buf[w.off:])

	return n, nil
}

func (w *datagramWriter) ensureSpace(n int) error {
	if w.off+n > w.size {
		if err := w.Flush(); err != nil {
			return err
		}
	}
	return nil
}

type datagramReader struct {
	channel *channel
}

func (v datagramReader) Read(b []byte) (n int, err error) {
	for {
		if n >= len(b) {
			return
		}

		mt := MessageType(b[n])
		n++

		var mn int
		switch mt {
		case HandshakeMessage:
			mn, err = v.readHandshake(b[n:])
		case DataMessage:
			mn, err = v.readData(b[n:])
		case AckMessage:
			mn, err = v.readAck(b[n:])
		case HaveMessage:
			mn, err = v.readHave(b[n:])
		case RequestMessage:
			mn, err = v.readRequest(b[n:])
		case CancelMessage:
			mn, err = v.readCancel(b[n:])
		case ChokeMessage:
			mn, err = v.readChoke(b[n:])
		case UnchokeMessage:
			mn, err = v.readUnchoke(b[n:])
		case PingMessage:
			mn, err = v.readPing(b[n:])
		case PongMessage:
			mn, err = v.readPong(b[n:])
		case EndMessage:
			return
		default:
			return n, ErrUnsupportedMessageType
		}

		if err != nil {
			return n, err
		}
		n += mn
	}
}

func (v datagramReader) readHandshake(b []byte) (int, error) {
	var msg Handshake
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	v.channel.HandleHandshake(msg)
	return n, err
}

func (v datagramReader) readData(b []byte) (int, error) {
	var msg Data
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	v.channel.HandleData(msg)
	return n, err
}

func (v datagramReader) readAck(b []byte) (int, error) {
	var msg Ack
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	v.channel.HandleAck(msg)
	return n, err
}

func (v datagramReader) readHave(b []byte) (int, error) {
	var msg Have
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	v.channel.HandleHave(msg)
	return n, err
}

func (v datagramReader) readRequest(b []byte) (int, error) {
	var msg Request
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	v.channel.HandleRequest(msg)
	return n, err
}

func (v datagramReader) readCancel(b []byte) (int, error) {
	var msg Cancel
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	v.channel.HandleCancel(msg)
	return n, err
}

func (v datagramReader) readChoke(b []byte) (int, error) {
	var msg Choke
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	v.channel.HandleChoke(msg)
	return n, err
}

func (v datagramReader) readUnchoke(b []byte) (int, error) {
	var msg Unchoke
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	v.channel.HandleUnchoke(msg)
	return n, err
}

func (v datagramReader) readPing(b []byte) (int, error) {
	var msg Ping
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	v.channel.HandlePing(msg)
	return n, err
}

func (v datagramReader) readPong(b []byte) (int, error) {
	var msg Pong
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	v.channel.HandlePong(msg)
	return n, err
}
