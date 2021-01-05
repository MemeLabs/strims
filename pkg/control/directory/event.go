package directory

import (
	"io"
	"io/ioutil"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/pkg/prefixstream"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/reflect/protoreflect"
)

var paddingData = make([]byte, chunkSize)
var paddingOverhead = proto.Size(&pb.DirectoryEvent{
	Body: &pb.DirectoryEvent_Padding_{
		Padding: &pb.DirectoryEvent_Padding{
			Data: paddingData,
		},
	},
}) - len(paddingData)

func newEventReader(r io.Reader) *eventReader {
	return &eventReader{
		r: prefixstream.NewReader(r),
	}
}

type eventReader struct {
	r *prefixstream.Reader
}

func (r *eventReader) ReadEvent(event *pb.DirectoryEvent) error {
	b, err := ioutil.ReadAll(r.r)
	if err != nil {
		return err
	}

	return proto.Unmarshal(b, event)
}

func newEventWriter(w io.Writer) *eventWriter {
	return &eventWriter{
		w: prefixstream.NewWriter(w),
	}
}

type eventWriter struct {
	w   io.Writer
	buf []byte
}

func (w *eventWriter) Write(msg protoreflect.ProtoMessage) error {
	n, err := w.write(msg)
	if err != nil {
		return err
	}

	_, err = w.write(&pb.DirectoryEvent{
		Body: &pb.DirectoryEvent_Padding_{
			Padding: &pb.DirectoryEvent_Padding{
				Data: paddingData[:chunkSize-(n%chunkSize)-paddingOverhead],
			},
		},
	})
	return err
}

func (w *eventWriter) write(msg protoreflect.ProtoMessage) (int, error) {
	b, err := proto.Marshal(msg)
	if err != nil {
		return 0, err
	}

	return w.w.Write(b)
}
