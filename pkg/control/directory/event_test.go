package directory

import (
	"bytes"
	"testing"

	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/stretchr/testify/assert"
)

func TestEventReadWriter(t *testing.T) {
	b := &testOffsetReadWriter{}

	w, err := newEventWriter(b)
	assert.NoError(t, err)
	r := newEventReader(b)

	src := &networkv1.DirectoryEventBroadcast{
		Events: []*networkv1.DirectoryEvent{
			{
				Body: &networkv1.DirectoryEvent_Ping_{
					Ping: &networkv1.DirectoryEvent_Ping{
						Time: 1257894000000000000,
					},
				},
			},
		},
	}

	for i := 0; i < 3; i++ {
		err = w.Write(src)
		assert.NoError(t, err)
	}

	for i := 0; i < 3; i++ {
		dst := &networkv1.DirectoryEventBroadcast{}
		err = r.Read(dst)
		assert.NoError(t, err)
		assert.Equal(t, src.Events[0].GetPing().Time, dst.Events[0].GetPing().Time)
	}
}

type testOffsetReadWriter struct {
	bytes.Buffer
}

func (r *testOffsetReadWriter) Offset() uint64 {
	return 0
}

func (r *testOffsetReadWriter) Flush() error {
	return nil
}
