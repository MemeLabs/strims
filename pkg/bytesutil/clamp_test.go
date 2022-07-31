// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package bytesutil

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestClamp(t *testing.T) {
	start := []byte("b:")
	end := append([]byte("b:"), 0xff)

	assert.EqualValues(t, start, Clamp([]byte("a:foo"), start, end))
	assert.EqualValues(t, []byte("b:foo"), Clamp([]byte("b:foo"), start, end))
	assert.EqualValues(t, end, Clamp([]byte("d:foo"), start, end))
}
