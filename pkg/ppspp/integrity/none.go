package integrity

import (
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
)

var noneChannelVerifier = &NoneChannelVerifier{}
var noneChunkVerifier = &NoneChunkVerifier{}

// NoneSwarmVerifier ...
type NoneSwarmVerifier struct{}

// WriteIntegrity ...
func (v *NoneSwarmVerifier) WriteIntegrity(b binmap.Bin, m *binmap.Map, w Writer) (int, error) {
	return 0, nil
}

// ChannelVerifier ...
func (v *NoneSwarmVerifier) ChannelVerifier() ChannelVerifier {
	return noneChannelVerifier
}

// NoneChannelVerifier ...
type NoneChannelVerifier struct{}

// ChunkVerifier ...
func (v *NoneChannelVerifier) ChunkVerifier(b binmap.Bin) ChunkVerifier {
	return noneChunkVerifier
}

// NoneChunkVerifier ...
type NoneChunkVerifier struct{}

// SetSignedIntegrity ...
func (v *NoneChunkVerifier) SetSignedIntegrity(b binmap.Bin, ts time.Time, sig []byte) {}

// SetIntegrity ...
func (v *NoneChunkVerifier) SetIntegrity(b binmap.Bin, hash []byte) {}

// Verify ...
func (v *NoneChunkVerifier) Verify(b binmap.Bin, d []byte) (bool, error) {
	return true, nil
}
