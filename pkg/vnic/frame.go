package vnic

import (
	"encoding/binary"
	"errors"
	"io"
	"sync"

	"github.com/MemeLabs/go-ppspp/pkg/pool"
	"github.com/MemeLabs/go-ppspp/pkg/vnic/qos"
)

const frameHeaderLen = 4

// FrameHeader ...
type FrameHeader struct {
	Port   uint16
	Length uint16
}

// WriteTo ...
func (f FrameHeader) WriteTo(w io.Writer) (int64, error) {
	var t [4]byte
	f.Marshal(t[:])
	n, err := w.Write(t[:])
	return int64(n), err
}

// Marshal ...
func (f FrameHeader) Marshal(b []byte) int {
	binary.BigEndian.PutUint16(b, f.Port)
	binary.BigEndian.PutUint16(b[2:], f.Length)
	return frameHeaderLen
}

// ReadFrom ...
func (f *FrameHeader) ReadFrom(r io.Reader) (int64, error) {
	var t [4]byte
	n, err := io.ReadFull(r, t[:])
	f.Unmarshal(t[:])
	return int64(n), err
}

// Unmarshal ...
func (f *FrameHeader) Unmarshal(b []byte) int {
	f.Port = binary.BigEndian.Uint16(b)
	f.Length = binary.BigEndian.Uint16(b[2:])
	return frameHeaderLen
}

// Frame ...
type Frame struct {
	Header FrameHeader
	Body   []byte
	body   *[]byte
}

// WriteTo ...
func (f Frame) WriteTo(w io.Writer) (int64, error) {
	b := pool.Get(int(frameHeaderLen + f.Header.Length))
	n := f.Marshal(*b)
	_, err := w.Write(*b)
	pool.Put(b)
	return int64(n), err
}

// Marshal ...
func (f Frame) Marshal(b []byte) int {
	n := f.Header.Marshal(b)
	return n + copy(b[n:], f.Body)
}

// ReadFrom ...
func (f *Frame) ReadFrom(r io.Reader) (int64, error) {
	hn, err := f.Header.ReadFrom(r)
	if err != nil {
		return 0, err
	}
	f.body = pool.Get(int(f.Header.Length))
	f.Body = *f.body
	bn, err := io.ReadFull(r, f.Body)
	return hn + int64(bn), err
}

// Unmarshal ...
func (f *Frame) Unmarshal(b []byte) int {
	hlen := f.Header.Unmarshal(b)
	n := hlen + int(f.Header.Length)
	f.Body = b[hlen:n]
	return n
}

// Free ...
func (f *Frame) Free() {
	pool.Put(f.body)
	f.body = nil
	f.Body = nil
}

var errClosedFrameWriter = errors.New("write on closed frameReadWriter")

// NewFrameReadWriter ...
func NewFrameReadWriter(w Link, port uint16, qc *qos.Class) *FrameReadWriter {
	return &FrameReadWriter{
		FrameReader: NewFrameReader(w.MTU()),
		FrameWriter: NewFrameWriter(w, port, qc),
	}
}

// FrameReadWriter ...
type FrameReadWriter struct {
	*FrameReader
	*FrameWriter
}

// MTU ...
func (f *FrameReadWriter) MTU() int {
	return f.FrameReader.MTU()
}

// Close ...
func (f *FrameReadWriter) Close() error {
	f.FrameReader.Close()
	f.FrameWriter.Close()
	return nil
}

// NewFrameReader ...
func NewFrameReader(size int) *FrameReader {
	r, w := io.Pipe()
	return &FrameReader{
		r:    r,
		w:    w,
		size: size,
	}
}

// FrameReader ...
type FrameReader struct {
	r    *io.PipeReader
	w    *io.PipeWriter
	size int
}

// MTU ...
func (b *FrameReader) MTU() int {
	return b.size - frameHeaderLen
}

// HandleFrame ...
func (b *FrameReader) HandleFrame(p *Peer, f Frame) error {
	_, err := b.w.Write(f.Body)
	return err
}

// Read ...
func (b *FrameReader) Read(p []byte) (int, error) {
	return b.r.Read(p)
}

// Close ...
func (b *FrameReader) Close() error {
	b.r.Close()
	b.w.Close()
	return nil
}

// NewFrameWriter ...
func NewFrameWriter(w Link, port uint16, qc *qos.Class) *FrameWriter {
	return &FrameWriter{
		w:           w,
		port:        port,
		size:        w.MTU(),
		writeBuffer: make([]byte, w.MTU()),
		off:         frameHeaderLen,
		close:       make(chan struct{}),
		qs:          qc.AddSession(1),
		qp: frameWriterPacket{
			ch: make(chan struct{}, 1),
		},
	}
}

// FrameWriter ...
type FrameWriter struct {
	w           io.Writer
	port        uint16
	size        int
	writeBuffer []byte
	off         int
	closed      bool
	close       chan struct{}
	closeOnce   sync.Once
	qs          *qos.Session
	qp          frameWriterPacket
}

// MTU ...
func (b *FrameWriter) MTU() int {
	return b.size - frameHeaderLen
}

// Port ...
func (b *FrameWriter) Port() uint16 {
	return b.port
}

// WriteFrame ...
func (b *FrameWriter) WriteFrame(p []byte) (int, error) {
	n, err := b.Write(p)
	if err != nil {
		return 0, err
	}
	if err := b.Flush(); err != nil {
		return n, err
	}
	return n, nil
}

// Write ...
func (b *FrameWriter) Write(p []byte) (int, error) {
	if b.closed {
		return 0, errClosedFrameWriter
	}

	n := len(p)
	for {
		l := copy(b.writeBuffer[b.off:], p)
		p = p[l:]
		b.off += l

		if b.off < len(b.writeBuffer) {
			return n, nil
		}

		if err := b.Flush(); err != nil {
			return n - len(p), err
		}
	}
}

// Flush ...
func (b *FrameWriter) Flush() error {
	if b.off == frameHeaderLen {
		return nil
	}

	h := FrameHeader{
		Port:   b.port,
		Length: uint16(b.off) - frameHeaderLen,
	}
	h.Marshal(b.writeBuffer)

	b.qp.size = uint64(b.off)
	b.qs.Enqueue(&b.qp)
	select {
	case <-b.qp.ch:
	case <-b.close:
		return errClosedFrameWriter
	}

	_, err := b.w.Write(b.writeBuffer[:b.off])
	if err == nil {
		b.off = frameHeaderLen
	}

	return err
}

// Close ...
func (b *FrameWriter) Close() error {
	b.closeOnce.Do(func() {
		b.closed = true
		close(b.close)
		b.qs.Close()
	})
	return nil
}

// SetQOSWeight ...
func (b *FrameWriter) SetQOSWeight(w uint64) {
	b.qs.SetWeight(w)
}

type frameWriterPacket struct {
	size uint64
	ch   chan struct{}
}

func (p *frameWriterPacket) Size() uint64 {
	return p.size
}

func (p *frameWriterPacket) Send() {
	p.ch <- struct{}{}
}
