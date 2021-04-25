package codec

import (
	"fmt"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
)

type EncoderDecoder interface {
	Encoder
	Decoder
	ByteLen() int
}

func TestMessageMarshalUnmarshal(t *testing.T) {
	cases := []struct {
		src      EncoderDecoder
		dst      EncoderDecoder
		expected EncoderDecoder
	}{
		{
			src: NewAddress(1234),
			dst: NewAddress(0),
		},
		{
			src: &Buffer{0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff},
			dst: &Buffer{},
		},
		{
			src: &VersionProtocolOption{Value: 123},
			dst: &VersionProtocolOption{Value: 123},
		},
		{
			src: &MinimumVersionProtocolOption{Value: 123},
			dst: &MinimumVersionProtocolOption{Value: 123},
		},
		{
			src: &LiveWindowProtocolOption{Value: 1 << 20},
			dst: &LiveWindowProtocolOption{Value: 1 << 20},
		},
		{
			src: &ChunkSizeProtocolOption{Value: 1024},
			dst: &ChunkSizeProtocolOption{Value: 1024},
		},
		{
			src: &ChunksPerSignatureProtocolOption{Value: 64},
			dst: &ChunksPerSignatureProtocolOption{Value: 64},
		},
		{
			src: &ContentIntegrityProtectionMethodProtocolOption{Value: 1},
			dst: &ContentIntegrityProtectionMethodProtocolOption{Value: 1},
		},
		{
			src: &MerkleHashTreeFunctionProtocolOption{Value: 1},
			dst: &MerkleHashTreeFunctionProtocolOption{Value: 1},
		},
		{
			src: &LiveSignatureAlgorithmProtocolOption{Value: 1},
			dst: &LiveSignatureAlgorithmProtocolOption{Value: 1},
		},
		{
			src: &SwarmIdentifierProtocolOption{0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff},
			dst: &SwarmIdentifierProtocolOption{},
		},
		{
			src: &Handshake{
				Options: ProtocolOptions{
					&VersionProtocolOption{Value: 123},
					&MinimumVersionProtocolOption{Value: 123},
					&LiveWindowProtocolOption{Value: 1 << 20},
					&ChunkSizeProtocolOption{Value: 1024},
					&ChunksPerSignatureProtocolOption{Value: 64},
					&ContentIntegrityProtectionMethodProtocolOption{Value: 1},
					&MerkleHashTreeFunctionProtocolOption{Value: 1},
					&LiveSignatureAlgorithmProtocolOption{Value: 1},
					&SwarmIdentifierProtocolOption{0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff},
				},
			},
			dst: &Handshake{},
		},
		{
			src: &Data{
				Address:   Address(22),
				Timestamp: Timestamp{Time: time.Unix(1234, 1234)},
				Data:      Buffer{0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff},
			},
			dst: &Data{
				chunkSize: 16,
			},
			expected: &Data{
				chunkSize: 16,
				Address:   Address(22),
				Timestamp: Timestamp{Time: time.Unix(1234, 1234)},
				Data:      Buffer{0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff},
			},
		},
		{
			src: &Timestamp{Time: time.Unix(1234, 1234)},
			dst: &Timestamp{},
		},
		{
			src: &DelaySample{Duration: time.Duration(1234)},
			dst: &DelaySample{},
		},
		{
			src: &Ack{
				Address:     Address(22),
				DelaySample: DelaySample{Duration: time.Duration(1234)},
			},
			dst: &Ack{},
		},
		{
			src: &Integrity{
				Address: Address(22),
				Hash:    Buffer{0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff},
			},
			dst: &Integrity{
				hashSize: 16,
			},
			expected: &Integrity{
				hashSize: 16,
				Address:  Address(22),
				Hash:     Buffer{0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff},
			},
		},
		{
			src: &SignedIntegrity{
				Address:   Address(22),
				Timestamp: Timestamp{Time: time.Unix(1234, 1234)},
				Signature: Buffer{0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff},
			},
			dst: &SignedIntegrity{
				signatureSize: 16,
			},
			expected: &SignedIntegrity{
				signatureSize: 16,
				Address:       Address(22),
				Timestamp:     Timestamp{Time: time.Unix(1234, 1234)},
				Signature:     Buffer{0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff},
			},
		},
		{
			src: &Nonce{Value: 1234},
			dst: &Nonce{},
		},
		{
			src: &Ping{Nonce{Value: 1234}},
			dst: &Ping{},
		},
		{
			src: &Pong{Nonce: Nonce{Value: 1234}},
			dst: &Pong{},
		},
		{
			src: &Have{Address(1234)},
			dst: &Have{},
		},
		{
			src: &Request{Address(1234)},
			dst: &Request{},
		},
		{
			src: &Cancel{Address(1234)},
			dst: &Cancel{},
		},
		{
			src:      NewStream(1234),
			dst:      NewStream(0),
			expected: NewStream(1234),
		},
		{
			src: &StreamAddress{
				Stream:  Stream(16),
				Address: Address(1234),
			},
			dst: &StreamAddress{},
		},
		{
			src: &StreamRequest{
				StreamAddress{
					Stream:  Stream(16),
					Address: Address(1234),
				},
			},
			dst: &StreamRequest{},
		},
		{
			src: &StreamCancel{Stream(16)},
			dst: &StreamCancel{},
		},
		{
			src: &StreamOpen{
				StreamAddress{
					Stream:  Stream(16),
					Address: Address(1234),
				},
			},
			dst: &StreamOpen{},
		},
		{
			src: &StreamClose{Stream(16)},
			dst: &StreamClose{},
		},
		{
			src: &Empty{},
			dst: &Empty{},
		},
		{
			src: &Choke{},
			dst: &Choke{},
		},
		{
			src: &Unchoke{},
			dst: &Unchoke{},
		},
		{
			src: &End{},
			dst: &End{},
		},
		{
			src: NewChannel(1234),
			dst: NewChannel(0),
		},
		{
			src: &ChannelHeader{
				Channel: Channel(16),
				Length:  16384,
			},
			dst: &ChannelHeader{},
		},
	}

	for _, c := range cases {
		t.Run(fmt.Sprintf("%T", c.src), func(t *testing.T) {
			b := make([]byte, c.src.ByteLen())

			assert.Equal(t, len(b), c.src.Marshal(b), "number of bytes written by Marshal should match ByteLen")

			n, err := c.dst.Unmarshal(b)
			assert.Equal(t, n, len(b), "Unmarshal should read the same number of bytes written by Marshal")
			assert.Nil(t, err, "Unmarshal should not return error")

			expected := c.expected
			if expected == nil {
				expected = c.src
			}
			assert.Equal(t, expected, c.dst, "Unmarshalled message should match expected")
		})
	}
}
