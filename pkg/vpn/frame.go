package vpn

import (
	"encoding/binary"
	"errors"
	"io"
	"math/bits"
	"sync"
)

var errBufferTooSmall = errors.New("buffer too small")

const frameBufferPoolCount = 8

var frameBufferPools = [frameBufferPoolCount]*sync.Pool{}

func newFrameBufferPool(size int) *sync.Pool {
	return &sync.Pool{
		New: func() interface{} {
			return make([]byte, size)
		},
	}
}

func init() {
	for i := 0; i < frameBufferPoolCount; i++ {
		frameBufferPools[i] = newFrameBufferPool(1<<(16-i) - 1)
	}
}

func frameBuffer(size uint16) (b []byte) {
	if i := bits.LeadingZeros16(size); i < frameBufferPoolCount {
		b = frameBufferPools[i].Get().([]byte)
	} else {
		b = frameBufferPools[frameBufferPoolCount-1].Get().([]byte)
	}
	return b[:size]
}

func freeFrameBuffer(b []byte) {
	if i := bits.LeadingZeros16(uint16(cap(b))); i < frameBufferPoolCount {
		frameBufferPools[i].Put(b)
	} else {
		frameBufferPools[frameBufferPoolCount-1].Put(b)
	}
}

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
}

// WriteTo ...
func (f Frame) WriteTo(w io.Writer) (int64, error) {
	b := frameBuffer(frameHeaderLen + f.Header.Length)
	n := f.Marshal(b)
	_, err := w.Write(b[:n])
	freeFrameBuffer(b)
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
	f.Body = frameBuffer(f.Header.Length)[:f.Header.Length]
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
	freeFrameBuffer(f.Body)
	f.Body = nil
}

var errClosedFrameWriter = errors.New("write on closed frameReadWriter")

// NewFrameReadWriter ...
func NewFrameReadWriter(w io.Writer, port uint16, size int) *FrameReadWriter {
	return &FrameReadWriter{
		FrameReader: NewFrameReader(size),
		FrameWriter: NewFrameWriter(w, port, size),
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
	readReader, readWriter := io.Pipe()
	return &FrameReader{
		readReader: readReader,
		readWriter: readWriter,
		size:       size,
	}
}

// FrameReader ...
type FrameReader struct {
	readReader *io.PipeReader
	readWriter *io.PipeWriter
	size       int
}

// MTU ...
func (b *FrameReader) MTU() int {
	return b.size - frameHeaderLen
}

// HandleFrame ...
func (b *FrameReader) HandleFrame(p *Peer, f Frame) error {
	_, err := b.readWriter.Write(f.Body)
	return err
}

// Read ...
func (b *FrameReader) Read(p []byte) (int, error) {
	return b.readReader.Read(p)
}

// Close ...
func (b *FrameReader) Close() error {
	b.readReader.Close()
	b.readWriter.Close()
	return nil
}

// NewFrameWriter ...
func NewFrameWriter(w io.Writer, port uint16, size int) *FrameWriter {
	return &FrameWriter{
		w:           w,
		port:        port,
		size:        size,
		writeBuffer: make([]byte, size),
		off:         frameHeaderLen,
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
}

// MTU ...
func (b *FrameWriter) MTU() int {
	return b.size - frameHeaderLen
}

// WriteFrame ...
func (b *FrameWriter) WriteFrame(p []byte) (int, error) {
	n, err := b.write(p)
	if err != nil {
		return 0, err
	}
	if err := b.flush(); err != nil {
		return n, err
	}
	return n, nil
}

// Write ...
func (b *FrameWriter) Write(p []byte) (int, error) {
	return b.write(p)
}

func (b *FrameWriter) write(p []byte) (int, error) {
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

		if err := b.flush(); err != nil {
			return n - len(p), err
		}
	}
}

// Flush ...
func (b *FrameWriter) Flush() error {
	return b.flush()
}

func (b *FrameWriter) flush() error {
	if b.off == frameHeaderLen {
		return nil
	}

	h := FrameHeader{
		Port:   b.port,
		Length: uint16(b.off) - frameHeaderLen,
	}
	h.Marshal(b.writeBuffer)

	_, err := b.w.Write(b.writeBuffer[:b.off])
	if err == nil {
		b.off = frameHeaderLen
	}

	return err
}

// Close ...
func (b *FrameWriter) Close() error {
	b.closed = true
	return nil
}
