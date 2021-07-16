package vpn

import (
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
	"github.com/stretchr/testify/assert"
	"go.uber.org/zap"
)

func host(t *testing.T) *vnic.Host {
	logger, err := zap.NewDevelopment()
	assert.Nil(t, err)
	key, err := dao.GenerateKey()
	assert.Nil(t, err)
	host, err := vnic.New(logger, key)
	assert.Nil(t, err)
	return host
}

func TestMarshalUnmarshal(t *testing.T) {
	msg := Message{
		Header: MessageHeader{
			DstID:   kademlia.ID{0xffffffffffffffff, 0xffffffffffffffff, 0xffffffffffffffff, 0xffffffffffffffff},
			DstPort: 101,
			SrcPort: 102,
			Seq:     103,
			Length:  4,
			Flags:   MstdFlags,
		},
		Body: []byte("test"),
	}
	b := make([]byte, msg.Size())
	msg.Marshal(b, host(t))

	msg1 := Message{}
	msg1.Unmarshal(b)
	msg1.Trailer = MessageTrailer{}

	assert.ObjectsAreEqualValues(msg, msg1)
}

func TestVerify(t *testing.T) {
	host0 := host(t)
	host1 := host(t)

	msg0 := Message{
		Header: MessageHeader{
			DstID:   kademlia.ID{0xffffffffffffffff, 0xffffffffffffffff, 0xffffffffffffffff, 0xffffffffffffffff},
			DstPort: 101,
			SrcPort: 102,
			Seq:     103,
			Length:  4,
			Flags:   MstdFlags,
		},
		Body: []byte("test"),
	}
	b := make([]byte, msg0.Size())
	msg0.Marshal(b, host0)

	msg1 := Message{}
	msg1.Unmarshal(b)

	b = make([]byte, msg1.Size())
	msg1.Marshal(b, host1)

	msg2 := Message{}
	msg2.Unmarshal(b)

	assert.True(t, msg2.Verify(0))
	assert.True(t, msg2.Verify(1))
}
