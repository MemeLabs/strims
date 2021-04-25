package ppspp

import (
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/stretchr/testify/assert"
)

func TestSwarmOptions(t *testing.T) {
	tcs := map[string]struct {
		req      SwarmOptions
		expected SwarmOptions
	}{
		"default options": {
			req:      SwarmOptions{},
			expected: NewDefaultSwarmOptions(),
		},
		"custom options": {
			req: SwarmOptions{
				ChunkSize:          2048,
				ChunksPerSignature: 16,
				LiveWindow:         1 << 12,
				Integrity: integrity.VerifierOptions{
					ProtectionMethod:       integrity.ProtectionMethodSignAll,
					MerkleHashTreeFunction: integrity.MerkleHashTreeFunctionSHA256,
					LiveSignatureAlgorithm: integrity.LiveSignatureAlgorithmED25519,
				},
			},
			expected: SwarmOptions{
				ChunkSize:          2048,
				ChunksPerSignature: 16,
				LiveWindow:         1 << 12,
				Integrity: integrity.VerifierOptions{
					ProtectionMethod:       integrity.ProtectionMethodSignAll,
					MerkleHashTreeFunction: integrity.MerkleHashTreeFunctionSHA256,
					LiveSignatureAlgorithm: integrity.LiveSignatureAlgorithmED25519,
				},
			},
		},
	}

	for scenario, tc := range tcs {
		t.Run(scenario, func(t *testing.T) {
			assert := assert.New(t)

			opt := NewDefaultSwarmOptions()
			opt.Assign(tc.req)

			assert.Equal(tc.expected, opt)
		})
	}
}
