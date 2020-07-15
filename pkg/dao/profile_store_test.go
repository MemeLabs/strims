package dao

import (
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/memkv"
	"github.com/tj/assert"
)

func createProfileStore(t *testing.T) (*ProfileStore, error) {
	t.Helper()

	profile, err := NewProfile("jbpratt")
	if err != nil {
		return nil, err
	}

	key, err := NewStorageKey("majoraautumn")
	if err != nil {
		return nil, err
	}

	kvStore, err := memkv.NewStore("strims")
	if err != nil {
		return nil, err
	}
	pfStore := NewProfileStore(1, kvStore, key)
	if err := pfStore.Init(profile); err != nil {
		return nil, err
	}

	return pfStore, nil
}

func TestInit(t *testing.T) {
	_, err := createProfileStore(t)
	assert.NoError(t, err, "failed to setup profile store")
}

func TestDeleteProfileStore(t *testing.T) {
	pfStore, err := createProfileStore(t)
	assert.NoError(t, err, "failed to setup profile store")
	assert.NoError(t, pfStore.Delete(), "failed to delete profile store")
	_, err = GetProfile(pfStore)
	assert.Error(t, err, "bucket not found: %s", pfStore.name)
}

func TestGetProfile(t *testing.T) {
	pfStore, err := createProfileStore(t)
	assert.NoError(t, err, "failed to setup profile store")

	profile, err := GetProfile(pfStore)
	assert.NoError(t, err, "failed to get profile")
	assert.Equal(t, profile.GetName(), "jbpratt")
	assert.NotNil(t, profile.GetKey())
}
