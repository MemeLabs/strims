// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package kvtest

import (
	"fmt"
	"testing"

	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/stretchr/testify/assert"
)

func TestScanCursor(t *testing.T) {
	s := NewMemStore()
	s.CreateStoreIfNotExists("test")

	s.Update("test", func(tx kv.BlobTx) error {
		for i := 0; i < 100; i++ {
			tx.Put(fmt.Sprintf("foo:%05d", i), []byte{byte(i)})
		}
		return nil
	})

	var before, after [][]byte
	s.View("test", func(tx kv.BlobTx) error {
		before, _ = tx.ScanCursor(kv.Cursor{After: "foo:", Before: "foo:00038", Last: 5})
		after, _ = tx.ScanCursor(kv.Cursor{After: "foo:00038", Before: "foo:\uffff", First: 10})
		return nil
	})

	assert.EqualValues(t, 5, len(before))
	assert.EqualValues(t, 10, len(after))
	for i, v := range before {
		assert.EqualValues(t, 37-i, v[0])
	}
	for i, v := range after {
		assert.EqualValues(t, 39+i, v[0])
	}
}
