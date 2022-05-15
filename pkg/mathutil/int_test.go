// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package mathutil

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestMinMaxInt(t *testing.T) {
	assert.EqualValues(t, 0, Min([]int{0, 1, 2}...))
	assert.EqualValues(t, 2, Max([]int{0, 1, 2}...))
	assert.EqualValues(t, 0, Min([]uint{0, 1, 2}...))
	assert.EqualValues(t, 2, Max([]uint{0, 1, 2}...))
}
