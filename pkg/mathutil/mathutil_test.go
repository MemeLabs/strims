// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package mathutil

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestLogisticFunc(t *testing.T) {
	fn := LogisticFunc(0, 1, 1)
	assert.EqualValues(t, 0, fn(-1000))
	assert.EqualValues(t, 0.5, fn(0))
	assert.EqualValues(t, 1, fn(1000))
}
