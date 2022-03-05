package protoutil

import (
	"bytes"
	"encoding/binary"
	"errors"
	"io"

	"github.com/MemeLabs/go-ppspp/pkg/binaryutil"
	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/MemeLabs/go-ppspp/pkg/ioutil"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/reflect/protoreflect"
)

const maxChunkStreamSize = 10 * 1024 * 1024

var ErrMaxChunkStreamSize = errors.New("message exceeds max segment size")

type OffsetReader interface {
	io.Reader
	Offset() uint64
}

func NewChunkStreamReader(or OffsetReader, size int) *ChunkStreamReader {
	return &ChunkStreamReader{
		or:   or,
		size: size,
	}
}

type ChunkStreamReader struct {
	or   OffsetReader
	size int
	zpr  *chunkstream.ZeroPadReader
	buf  bytes.Buffer
}

func (r *ChunkStreamReader) Read(m protoreflect.ProtoMessage) error {
	if r.zpr == nil {
		off := r.or.Offset()
		var err error
		r.zpr, err = chunkstream.NewZeroPadReaderSize(r.or, int64(off), r.size)
		if err != nil {
			return err
		}
	}

	for {
		r.buf.Reset()
		_, err := r.buf.ReadFrom(io.LimitReader(r.zpr, maxChunkStreamSize))
		if err != nil {
			return err
		}

		size, err := binary.ReadUvarint(ioutil.NewByteReader(&r.buf))
		if err != nil {
			return err
		}

		if int(size) == r.buf.Len() {
			return proto.Unmarshal(r.buf.Bytes(), m)
		}
	}
}

func NewChunkStreamWriter(w ioutil.WriteFlusher, size int) (*ChunkStreamWriter, error) {
	zpw, err := chunkstream.NewZeroPadWriterSize(w, size)
	if err != nil {
		return nil, err
	}
	return &ChunkStreamWriter{
		zpw: zpw,
		buf: make([]byte, binary.MaxVarintLen64),
	}, nil
}

type ChunkStreamWriter struct {
	zpw *chunkstream.ZeroPadWriter
	buf []byte
}

func (w *ChunkStreamWriter) Size(m protoreflect.ProtoMessage) int {
	opt := proto.MarshalOptions{
		UseCachedSize: true,
	}

	n := opt.Size(m)
	n += binaryutil.UvarintLen(uint64(n))
	return n + w.zpw.Overhead(n)
}

func (w *ChunkStreamWriter) Write(m protoreflect.ProtoMessage) error {
	opt := proto.MarshalOptions{
		UseCachedSize: true,
	}

	size := opt.Size(m)
	if size > maxChunkStreamSize {
		return ErrMaxChunkStreamSize
	}
	n := binary.PutUvarint(w.buf, uint64(size))

	var err error
	w.buf, err = opt.MarshalAppend(w.buf[:n], m)
	if err != nil {
		return err
	}

	if _, err = w.zpw.Write(w.buf); err != nil {
		return err
	}
	return w.zpw.Flush()
}
