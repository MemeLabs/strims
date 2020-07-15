package dao

import (
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/memkv"
	"github.com/stretchr/testify/assert"
)

func createMetadataStore(t *testing.T) *MetadataStore {
	t.Helper()

	kvStore, err := memkv.NewStore("strims")
	assert.Nil(t, err, "failed to create kvstore")

	mdStore, err := NewMetadataStore(kvStore)
	assert.Nil(t, err, "failed to metadata store")

	return mdStore
}

func TestCreateProfile(t *testing.T) {
	mdStore := createMetadataStore(t)

	name := "jbpratt"
	profile, profileStore, err := CreateProfile(mdStore, name, "autumnmajora")
	assert.Nil(t, err, "failed to create profile")
	assert.NotNil(t, profile)
	assert.NotNil(t, profileStore)

	assert.Equal(t, profile.GetName(), name)
}

func TestCreateProfileUsernameTaken(t *testing.T) {
	mdStore := createMetadataStore(t)

	name := "jbpratt"
	_, _, err := CreateProfile(mdStore, name, "autumnmajora")
	assert.Nil(t, err, "failed to create profile")

	_, _, err = CreateProfile(mdStore, name, "autumnmajora")
	assert.EqualError(t, err, ErrProfileNameNotAvailable.Error())
}

func TestDeleteProfile(t *testing.T) {
	mdStore := createMetadataStore(t)

	profile, _, err := CreateProfile(mdStore, "jbpratt", "autumnmajora")
	assert.Nil(t, err, "failed to create profile")
	assert.Nil(t, DeleteProfile(mdStore, profile), "failed to delete profile")
}

func TestGetProfileSummaries(t *testing.T) {
	mdStore := createMetadataStore(t)

	_, _, err := CreateProfile(mdStore, "jbpratt", "autumnmajora")
	assert.Nil(t, err, "failed to create profile")
	_, _, err = CreateProfile(mdStore, "autumn", "jbprattmajora")
	assert.Nil(t, err, "failed to create profile")
	_, _, err = CreateProfile(mdStore, "majora", "jbprattautumn")
	assert.Nil(t, err, "failed to create profile")

	summaries, err := GetProfileSummaries(mdStore)
	assert.Nil(t, err, "failed to get profile summaries")

	assert.Equal(t, 3, len(summaries))
}
