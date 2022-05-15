// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package prefixstream

import (
	"bytes"
	"io"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestE2E(t *testing.T) {
	b := bytes.NewBuffer(nil)
	w := NewWriter(b)
	r := NewReader(b)

	ns := []int{27, 100000, 128}

	for _, n := range ns {
		_, err := w.Write(make([]byte, n))
		assert.Nil(t, err)
	}

	for _, n := range ns {
		rn, err := r.Read(make([]byte, n))
		if err != io.EOF || n != rn {
			t.Errorf("expected to read %d, read %d", n, rn)
			t.FailNow()
		}
	}
}
