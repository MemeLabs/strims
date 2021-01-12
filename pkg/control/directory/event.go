package directory

import (
	"io"
	"io/ioutil"

	network "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/MemeLabs/go-ppspp/pkg/prefixstream"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/reflect/protoreflect"
)

var paddingData = make([]byte, chunkSize)
var paddingOverhead = proto.Size(&network.DirectoryEvent{
	Body: &network.DirectoryEvent_Padding_{
		Padding: &network.DirectoryEvent_Padding{
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

func (r *eventReader) ReadEvent(event *network.DirectoryEvent) error {
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

	_, err = w.write(&network.DirectoryEvent{
		Body: &network.DirectoryEvent_Padding_{
			Padding: &network.DirectoryEvent_Padding{
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
