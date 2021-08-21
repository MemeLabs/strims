package vpn

import (
	"strings"
	"testing"

	"github.com/MemeLabs/go-ppspp/internal/dao"
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

func TestCompressUncompress(t *testing.T) {
	body := []byte(strings.Repeat("some message large enough to be compressed", 64))
	msg := Message{
		Header: MessageHeader{
			Flags:  Mcompress,
			Length: uint16(len(body)),
		},
		Body: body,
	}

	var hs networkMessageHandler = func(n *Network, m *Message) error {
		assert.ObjectsAreEqualValues(msg, m)
		assert.EqualValues(t, len(m.Body), m.Header.Length)
		return nil
	}
	hs = stackNetworkMessageHandler(hs, uncompressMessage)
	hs = stackNetworkMessageHandler(hs, func(n *Network, m *Message, next networkMessageHandler) error {
		assert.GreaterOrEqual(t, len(body), len(m.Body))
		assert.EqualValues(t, len(m.Body), m.Header.Length)
		return next(n, m)
	})
	hs = stackNetworkMessageHandler(hs, compressMessage)

	hs(nil, &msg)
}
