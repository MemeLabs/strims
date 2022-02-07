package dao

import (
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/kv/kvtest"
	"github.com/stretchr/testify/assert"
)

func createProfileStore(t *testing.T) *ProfileStore {
	t.Helper()

	profile, err := NewProfile("testuser")
	assert.Nil(t, err, "failed to create profile")

	key, err := NewStorageKey("majoraautumn")
	assert.Nil(t, err, "failed to storage key")

	kvStore := kvtest.NewMemStore()

	pfStore := NewProfileStore(profile.Id, key, kvStore, nil)
	assert.Nil(t, pfStore.Init(), "failed to create profile store")

	return pfStore
}

func TestInit(t *testing.T) {
	assert.NotNil(t, createProfileStore(t), "failed to setup profile store")
}

func TestDeleteProfileStore(t *testing.T) {
	pfStore := createProfileStore(t)
	assert.Nil(t, pfStore.Delete(), "failed to delete profile store")
	_, err := Profile.Get(pfStore)
	assert.NotNilf(t, err, "bucket not found: %s", pfStore.name)
}
