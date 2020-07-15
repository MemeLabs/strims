package dao

import (
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/memkv"
	"github.com/stretchr/testify/assert"
)

func createProfileStore(t *testing.T) *ProfileStore {
	t.Helper()

	profile, err := NewProfile("jbpratt")
	assert.Nil(t, err, "failed to create profile")

	key, err := NewStorageKey("majoraautumn")
	assert.Nil(t, err, "failed to storage key")

	kvStore, err := memkv.NewStore("strims")
	assert.Nil(t, err, "failed to kv store")

	pfStore := NewProfileStore(1, kvStore, key)
	assert.Nil(t, pfStore.Init(profile), "failed to create profile store")

	return pfStore
}

func TestInit(t *testing.T) {
	assert.NotNil(t, createProfileStore(t), "failed to setup profile store")
}

func TestDeleteProfileStore(t *testing.T) {
	pfStore := createProfileStore(t)
	assert.Nil(t, pfStore.Delete(), "failed to delete profile store")
	_, err := GetProfile(pfStore)
	assert.NotNilf(t, err, "bucket not found: %s", pfStore.name)
}

func TestGetProfile(t *testing.T) {
	pfStore := createProfileStore(t)
	profile, err := GetProfile(pfStore)
	assert.Nil(t, err, "failed to get profile")
	assert.Equal(t, profile.GetName(), "jbpratt")
	assert.NotNil(t, profile.GetKey())
}
