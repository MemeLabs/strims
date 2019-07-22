package encoding

import (
	"context"
	"io"
)

type MemeWriter struct {
	d *Datagram
	t Transport
	c TransportConn
}

func (w MemeWriter) SetChannelID(id uint32) {
	w.d.ChannelID = id
}

func (w MemeWriter) Write(m Message) {
	// TODO: check byte length sum against t.MTU()
	w.d.Messages = append(w.d.Messages, m)
}

func (w MemeWriter) Flush() error {
	if len(w.d.Messages) == 0 {
		return nil
	}

	b := make([]byte, 1500)
	n := w.d.Marshal(b)
	// spew.Dump("writing", b[:n])
	return w.t.Write(b[:n], w.c)
}

type MemeRequest struct {
	Datagram
	Conn TransportConn
}

type MemeHandler func(w MemeWriter, r *MemeRequest)

type MemeServer struct {
	t       Transport
	Handler MemeHandler
}

func (s *MemeServer) Listen(ctx context.Context) (err error) {
	defer s.Shutdown()

	go func() {
		<-ctx.Done()
		s.Shutdown()
	}()

	if err = s.t.Listen(); err != nil {
		return
	}

	// TODO: writer cache for buffered output?

	for {
		if err = s.readDatagram(); err != nil {
			return
		}
	}
}

func (s *MemeServer) readDatagram() (err error) {
	// defer func() {
	// 	if err := recover(); err != nil {
	// 		// TODO: drop the peer?
	// 		fmt.Println(err)
	// 	}
	// }()

	b, c, err := s.t.Read()
	if err != nil {
		if err == io.EOF {
			return nil
		}
		return
	}

	r := &MemeRequest{Conn: c}
	if _, err = r.Datagram.Unmarshal(b); err != nil {
		return
	}

	w := MemeWriter{
		d: &Datagram{},
		t: s.t,
		c: c,
	}

	s.Handler(w, r)

	// TODO: reuse writer/defer flush
	w.Flush()

	return
}

func (s *MemeServer) Shutdown() (err error) {
	return s.t.Close()
}
