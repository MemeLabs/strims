package dao

import (
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/kv/kvtest"
	"github.com/stretchr/testify/assert"
)

func createMetadataStore(t *testing.T) *MetadataStore {
	t.Helper()

	kvStore, err := kvtest.NewMemStore("strims")
	assert.Nil(t, err, "failed to create kvstore")

	mdStore, err := NewMetadataStore(kvStore)
	assert.Nil(t, err, "failed to metadata store")

	return mdStore
}

func TestCreateProfile(t *testing.T) {
	mdStore := createMetadataStore(t)

	name := "testuser"
	profile, profileStore, err := CreateProfile(mdStore, name, "1234")
	assert.Nil(t, err, "failed to create profile")
	assert.NotNil(t, profile)
	assert.NotNil(t, profileStore)

	assert.Equal(t, profile.GetName(), name)
}

func TestCreateProfileUsernameTaken(t *testing.T) {
	mdStore := createMetadataStore(t)

	name := "testuser"
	_, _, err := CreateProfile(mdStore, name, "1234")
	assert.Nil(t, err, "failed to create profile")

	_, _, err = CreateProfile(mdStore, name, "1234")
	assert.EqualError(t, err, ErrProfileNameNotAvailable.Error())
}

func TestDeleteProfile(t *testing.T) {
	mdStore := createMetadataStore(t)

	profile, _, err := CreateProfile(mdStore, "testuser", "1234")
	assert.Nil(t, err, "failed to create profile")
	assert.Nil(t, DeleteProfile(mdStore, profile), "failed to delete profile")
}

func TestGetProfileSummaries(t *testing.T) {
	mdStore := createMetadataStore(t)

	_, _, err := CreateProfile(mdStore, "testuser", "1234")
	assert.Nil(t, err, "failed to create profile")
	_, _, err = CreateProfile(mdStore, "testuser1", "1234")
	assert.Nil(t, err, "failed to create profile")
	_, _, err = CreateProfile(mdStore, "testuser2", "1234")
	assert.Nil(t, err, "failed to create profile")

	summaries, err := GetProfileSummaries(mdStore)
	assert.Nil(t, err, "failed to get profile summaries")

	assert.Equal(t, 3, len(summaries))
}
