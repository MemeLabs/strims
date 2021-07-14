package vpn

import (
	"strings"
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/stretchr/testify/assert"
)

func TestMessageCipher(t *testing.T) {
	ak, _ := dao.GenerateKey()
	bk, _ := dao.GenerateKey()
	bid, _ := kademlia.UnmarshalID(bk.Public)

	c, err := newMessageCipher(ak, bid)
	assert.NoError(t, err)

	p := []byte("some test string")
	b := make([]byte, 0, len(p)+c.Overhead())
	b, err = c.Seal(b, p)
	assert.NoError(t, err)

	assert.NotEqualValues(t, p, b)

	b0 := make([]byte, 0, len(b))
	b0, err = c.Open(b0, b)
	assert.NoError(t, err)

	assert.EqualValues(t, p, b0)

	b1 := make([]byte, 0, len(p)+c.Overhead())
	b1, err = c.Seal(b1, p)
	assert.NoError(t, err)

	assert.NotEqualValues(t, b, b1)
	assert.NotEqualValues(t, p, b)
}

func TestMarshalUnmarshalCompressed(t *testing.T) {
	body := []byte(strings.Repeat("some message large enough to be compressed", 64))
	msg := Message{
		Header: MessageHeader{
			Flags:  Mcompress,
			Length: uint16(len(body)),
		},
		Body: body,
	}

	var b []byte
	var mhs networkMessageHandler = func(n *Network, m *Message) error {
		b = make([]byte, msg.Size())
		_, err := msg.Marshal(b, host(t))
		return err
	}
	mhs = stackNetworkMessageHandler(mhs, compressMessage)
	err := mhs(nil, &msg)
	assert.NoError(t, err)

	var uhs networkMessageHandler = func(n *Network, m *Message) error {
		m.Trailer = MessageTrailer{}
		assert.ObjectsAreEqualValues(msg, m)
		return nil
	}
	uhs = stackNetworkMessageHandler(uhs, decompressMessage)

	var msg1 Message
	_, err = msg1.Unmarshal(b)
	assert.NoError(t, err)

	err = uhs(nil, &msg1)
	assert.NoError(t, err)
}
