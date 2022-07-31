// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package bbolt

import (
	"encoding/base64"
	"fmt"
	"os"
	"path"
	"testing"

	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/stretchr/testify/assert"
)

func createTestDB(t *testing.T) kv.BlobStore {
	d, err := os.MkdirTemp("", "blobstore")
	assert.NoError(t, err)
	s, err := NewStore(path.Join(d, "test.db"))
	assert.NoError(t, err)
	s.CreateStoreIfNotExists("test")
	return s
}

func TestScanCursor(t *testing.T) {
	s := createTestDB(t)
	s.Update("test", func(tx kv.BlobTx) error {
		for i := 0; i < 255; i++ {
			tx.Put(fmt.Sprintf("foo:%s", base64.RawStdEncoding.EncodeToString([]byte{byte(i)})), []byte{byte(i)})
		}
		return nil
	})

	var before, after [][]byte
	s.View("test", func(tx kv.BlobTx) error {
		before, _ = tx.ScanCursor(kv.Cursor{Prefix: "foo:", Before: "foo:ZA", Last: 5})
		after, _ = tx.ScanCursor(kv.Cursor{After: "foo:ZA", Prefix: "foo:", First: 10})
		return nil
	})

	assert.EqualValues(t, 5, len(before))
	assert.EqualValues(t, 10, len(after))
	for i, v := range before {
		assert.EqualValues(t, 99-i, v[0], "results should be in descending order beginning before ZA (100)")
	}
	for i, v := range after {
		assert.EqualValues(t, 101+i, v[0], "results should be in ascending order beginning after ZA (100)")
	}
}

func TestScanCursorPrefix(t *testing.T) {
	s := createTestDB(t)
	s.Update("test", func(tx kv.BlobTx) error {
		for _, p := range []string{"a", "b", "c"} {
			for i := 0; i < 10; i++ {
				tx.Put(fmt.Sprintf("%s:%s", p, base64.RawStdEncoding.EncodeToString([]byte{byte(i)})), []byte{byte(i)})
			}
		}
		return nil
	})

	var before, after [][]byte
	s.View("test", func(tx kv.BlobTx) error {
		before, _ = tx.ScanCursor(kv.Cursor{Prefix: "b:", Before: "b:AQ", Last: 10})
		after, _ = tx.ScanCursor(kv.Cursor{After: "b:CA", Prefix: "b:", First: 10})
		return nil
	})

	assert.EqualValues(t, 1, len(before))
	assert.EqualValues(t, 1, len(after))
	assert.EqualValues(t, 0, before[0][0])
	assert.EqualValues(t, 9, after[0][0])
}
