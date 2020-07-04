package ppspp

import (
	"testing"

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
				ChunkSize:  2048,
				LiveWindow: 1 << 12,
			},
			expected: SwarmOptions{
				ChunkSize:  2048,
				LiveWindow: 1 << 12,
			},
		},
	}

	for scenario, tc := range tcs {
		t.Run(scenario, func(t *testing.T) {
			assert := assert.New(t)

			opt := NewDefaultSwarmOptions()
			opt.assign(tc.req)

			assert.Equal(tc.expected, opt)
		})
	}
}
