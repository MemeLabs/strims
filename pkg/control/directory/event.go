package directory

import (
	"bytes"
	"errors"
	"io"

	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/MemeLabs/go-ppspp/pkg/ioutil"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/reflect/protoreflect"
)

const maxEventBroadcastSize = 10 * 1024 * 1024

var errMaxEventBroadcastSize = errors.New("i/o exceeds max event bundle size")

type offsetReader interface {
	io.Reader
	Offset() uint64
}

func newEventReader(or offsetReader) *EventReader {
	return &EventReader{or: or}
}

type EventReader struct {
	or  offsetReader
	zpr *chunkstream.ZeroPadReader
	buf bytes.Buffer
}

func (r *EventReader) Read(m protoreflect.ProtoMessage) error {
	if r.zpr == nil {
		off := r.or.Offset()
		var err error
		r.zpr, err = chunkstream.NewZeroPadReaderSize(r.or, int64(off), chunkSize)
		if err != nil {
			return err
		}
	}

	r.buf.Reset()
	_, err := r.buf.ReadFrom(io.LimitReader(r.zpr, maxEventBroadcastSize))
	if err != nil {
		return err
	}

	return proto.Unmarshal(r.buf.Bytes(), m)
}

func newEventWriter(w ioutil.WriteFlusher) (*EventWriter, error) {
	zpw, err := chunkstream.NewZeroPadWriterSize(w, chunkSize)
	if err != nil {
		return nil, err
	}
	return &EventWriter{zpw: zpw}, nil
}

type EventWriter struct {
	zpw *chunkstream.ZeroPadWriter
	buf []byte
}

func (w *EventWriter) Write(m protoreflect.ProtoMessage) error {
	opt := proto.MarshalOptions{
		UseCachedSize: true,
	}
	if opt.Size(m) > maxEventBroadcastSize {
		return errMaxEventBroadcastSize
	}

	var err error
	w.buf, err = opt.MarshalAppend(w.buf[:0], m)
	if err != nil {
		return err
	}

	if _, err = w.zpw.Write(w.buf); err != nil {
		return err
	}
	return w.zpw.Flush()
}
