package integrity

import (
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
)

var noneChannelVerifier = &NoneChannelVerifier{}
var noneChunkVerifier = &NoneChunkVerifier{}

type NoneSwarmVerifier struct{}

func (v *NoneSwarmVerifier) WriteIntegrity(b binmap.Bin, m *binmap.Map, w IntegrityWriter) (int, error) {
	return 0, nil
}

func (v *NoneSwarmVerifier) ChannelVerifier() ChannelVerifier {
	return noneChannelVerifier
}

type NoneChannelVerifier struct{}

func (v *NoneChannelVerifier) ChunkVerifier(b binmap.Bin) ChunkVerifier {
	return noneChunkVerifier
}

type NoneChunkVerifier struct{}

func (v *NoneChunkVerifier) SetSignedIntegrity(b binmap.Bin, ts time.Time, sig []byte) {}

func (v *NoneChunkVerifier) SetIntegrity(b binmap.Bin, hash []byte) {}

func (v *NoneChunkVerifier) Verify(b binmap.Bin, d []byte) bool {
	return true
}
