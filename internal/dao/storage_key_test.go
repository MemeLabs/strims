// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package dao

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func createStorageKey(t *testing.T, password string) *StorageKey {
	t.Helper()

	key, err := NewStorageKey(password)
	assert.Nil(t, err, "failed to create storage key")
	assert.NotNil(t, key)

	return key
}

func TestNewStorageKey(t *testing.T) {
	createStorageKey(t, "sup3rs3cr3tp4ssw0rd")
}

func TestMarshalAndUnmarshalStorageKey(t *testing.T) {
	psswd := "sup3rs3cr3tp4ssw0rd"
	key := createStorageKey(t, psswd)

	bytes, err := MarshalStorageKey(key)
	assert.Nil(t, err, "failed to marshal storage key")
	assert.NotNil(t, bytes)

	unmarshaledKey, err := UnmarshalStorageKey(bytes, psswd)
	assert.Nil(t, err, "failed to unmarshal storage key")
	assert.NotNil(t, unmarshaledKey)

	assert.Equal(t, key.key, unmarshaledKey.key)
	assert.Equal(t, key.record.GetKdfType(), unmarshaledKey.record.GetKdfType())
}

func TestSealAndOpen(t *testing.T) {
	key := createStorageKey(t, "sup3rs3cr3tp4ssw0rd")
	data := []byte("autumnmajora")

	bytes, err := key.Seal(data)
	assert.Nil(t, err, "failed to seal data")
	assert.NotNil(t, bytes)

	unencrypted, err := key.Open(bytes)
	assert.Nil(t, err, "failed to open encrypted data")
	assert.NotNil(t, unencrypted)

	assert.Equal(t, data, unencrypted)
}
