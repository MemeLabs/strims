package integrity

import (
	swarmpb "github.com/MemeLabs/go-ppspp/pkg/apis/type/swarm"
	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
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

func (v *NoneSwarmVerifier) ImportCache(c *swarmpb.Cache) error {
	return nil
}

func (v *NoneSwarmVerifier) ExportCache() *swarmpb.Cache_Integrity {
	return &swarmpb.Cache_Integrity{}
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
func (v *NoneChunkVerifier) SetSignedIntegrity(b binmap.Bin, ts timeutil.Time, sig []byte) {}

// SetIntegrity ...
func (v *NoneChunkVerifier) SetIntegrity(b binmap.Bin, hash []byte) {}

// Verify ...
func (v *NoneChunkVerifier) Verify(b binmap.Bin, d []byte) (bool, error) {
	return true, nil
}
