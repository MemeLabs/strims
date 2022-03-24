package ioutil

func NewPrefixBufferWriter(w BufferedWriteFlusher, size int) *PrefixBufferWriter {
	return &PrefixBufferWriter{
		BufferedWriteFlusher: w,
		size:                 size,
	}
}

// PrefixBufferWriter intercepts requests for the underlying writer's buffer and
// reserves the first `size` bytes for prefix encoding.
type PrefixBufferWriter struct {
	BufferedWriteFlusher
	size int
	buf  []byte
}

func (b *PrefixBufferWriter) Size() int {
	return b.size
}

func (b *PrefixBufferWriter) PrefixBuffer() []byte {
	return b.buf
}

func (b *PrefixBufferWriter) Available() int {
	return b.BufferedWriteFlusher.Available() - b.size
}

func (b *PrefixBufferWriter) AvailableBuffer() []byte {
	buf := b.BufferedWriteFlusher.AvailableBuffer()
	if cap(buf) < b.size {
		b.buf = nil
		return nil
	}

	b.buf = buf[:b.size]
	return b.buf[b.size:][:0]
}
