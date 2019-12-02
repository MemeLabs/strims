package encoding

import (
	"context"
	"io"
	"log"
)

// NewMemeWriter ...
func NewMemeWriter(c TransportConn) *MemeWriter {
	return &MemeWriter{
		Conn: c,
		buf:  make([]byte, c.Transport().MTU()),
	}
}

// MemeWriter ...
type MemeWriter struct {
	Conn  TransportConn
	buf   []byte
	len   int
	cid   uint32
	dirty bool
}

// BeginFrame ...
func (w *MemeWriter) BeginFrame(channelID uint32) {
	if w.cid == channelID {
		return
	}

	if w.dirty {
		w.Write(&End{})
	}

	w.cid = channelID
	w.dirty = false
}

// Write ...
func (w *MemeWriter) Write(messages ...Message) {
	m := Messages(messages)

	expected := w.len + m.ByteLen()
	if !w.dirty {
		// datagram header length
		expected += 4
	}

	if expected >= w.Conn.Transport().MTU() {
		w.Flush()
	}

	if !w.dirty {
		w.dirty = true

		d := Datagram{ChannelID: w.cid}
		w.len += d.Marshal(w.buf[w.len:])
	}

	w.len += m.Marshal(w.buf[w.len:])
}

// Flush ...
func (w *MemeWriter) Flush() (err error) {
	if !w.dirty {
		return
	}
	err = w.Conn.Write(w.buf[:w.len])

	w.len = 0
	w.dirty = false

	return
}

// Dirty ...
func (w *MemeWriter) Dirty() bool {
	return w.dirty
}

// MemeRequest ...
type MemeRequest struct {
	Datagram
	Conn TransportConn
}

// MemeHandler ...
type MemeHandler func(w *MemeWriter, r *MemeRequest)

// MemeServer ...
type MemeServer struct {
	t        Transport
	Handler  MemeHandler
	readBuf  []byte
	writeBuf []byte

	// DecoderOptions func(cid uint32) (DecoderOptions, error)
}

// Listen ...
func (s *MemeServer) Listen(ctx context.Context) (err error) {
	defer s.Shutdown()

	go func() {
		<-ctx.Done()
		s.Shutdown()
	}()

	if err = s.t.Listen(ctx); err != nil {
		log.Println(err)
		return
	}

	buf := make([]byte, s.t.MTU()*2)
	s.readBuf = buf[:s.t.MTU()]
	s.writeBuf = buf[s.t.MTU():]

	for {
		if err = s.readDatagram(); err != nil {
			return
		}
	}
}

// readDatagram ...
func (s *MemeServer) readDatagram() (err error) {
	n, c, err := s.t.Read(s.readBuf)
	if err != nil {
		if err == io.EOF {
			return nil
		}
		return
	}

	r := MemeRequest{
		Conn: c,
	}
	w := MemeWriter{
		Conn: c,
		buf:  s.writeBuf,
	}

	for b := s.readBuf[:n]; len(b) != 0; {
		n, err = r.Datagram.Unmarshal(b)
		if err != nil {
			return
		}
		b = b[n:]
		s.Handler(&w, &r)
	}

	w.Flush()

	return
}

// Shutdown ...
func (s *MemeServer) Shutdown() (err error) {
	return s.t.Close()
}
