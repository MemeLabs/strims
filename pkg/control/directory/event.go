package directory

import (
	"io"

	network "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/MemeLabs/go-ppspp/pkg/ioutil"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/reflect/protoreflect"
)

type offsetReader interface {
	io.Reader
	Offset() uint64
}

func newEventReader(or offsetReader) *eventReader {
	return &eventReader{or: or}
}

type eventReader struct {
	or  offsetReader
	zpr *chunkstream.ZeroPadReader
}

func (r *eventReader) Read(event *network.DirectoryEvent) error {
	if r.zpr == nil {
		off := r.or.Offset()
		var err error
		r.zpr, err = chunkstream.NewZeroPadReaderSize(r.or, int64(off), chunkSize)
		if err != nil {
			return err
		}
	}

	b, err := io.ReadAll(r.zpr)
	if err != nil {
		return err
	}

	return proto.Unmarshal(b, event)
}

func newEventWriter(w ioutil.WriteFlusher) (*eventWriter, error) {
	zpw, err := chunkstream.NewZeroPadWriterSize(w, chunkSize)
	if err != nil {
		return nil, err
	}
	return &eventWriter{zpw: zpw}, nil
}

type eventWriter struct {
	zpw *chunkstream.ZeroPadWriter
	buf []byte
}

func (w *eventWriter) Write(m protoreflect.ProtoMessage) error {
	var err error
	w.buf, err = proto.MarshalOptions{}.MarshalAppend(w.buf[:0], m)
	if err != nil {
		return err
	}

	if _, err = w.zpw.Write(w.buf); err != nil {
		return err
	}
	return w.zpw.Flush()
}
