package vpn

import (
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/docker/docker/pkg/testutil/assert"
)

func TestMarshalUnmarshal(t *testing.T) {
	key, err := dao.GenerateKey()
	assert.NilError(t, err)
	host := &Host{key: key}

	msg := Message{
		Header: MessageHeader{
			DstID:   kademlia.ID{0xffffffffffffffff, 0xffffffffffffffff, 0xffffffffffffffff, 0xffffffffffffffff},
			DstPort: 101,
			SrcPort: 102,
			Seq:     103,
			Length:  4,
		},
		Body: []byte("test"),
	}
	b := make([]byte, msg.Size())
	msg.Marshal(b, host)

	msg1 := Message{}
	msg1.Unmarshal(b)
	msg1.Trailers = nil

	assert.DeepEqual(t, msg, msg1)
}
