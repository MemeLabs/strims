package dao

import (
	"testing"

	"github.com/tj/assert"
)

// TODO: test LoadSession

func createMetadataStore(t *testing.T) *MetadataStore {
	t.Helper()

	kvStore, err := NewKVStore("strims")
	if err != nil {
		t.Fatal(err)
	}

	mdStore, err := NewMetadataStore(kvStore)
	assert.NoError(t, err, "failed to metadata store")

	return mdStore
}

func TestCreateProfile(t *testing.T) {
	mdStore := createMetadataStore(t)

	name := "jbpratt"
	profile, profileStore, err := mdStore.CreateProfile(name, "autumnmajora")
	assert.NoError(t, err, "failed to create profile")
	assert.NotNil(t, profile)
	assert.NotNil(t, profileStore)

	assert.Equal(t, profile.GetName(), name)
}

func TestCreateProfileUsernameTaken(t *testing.T) {
	mdStore := createMetadataStore(t)

	name := "jbpratt"
	_, _, err := mdStore.CreateProfile(name, "autumnmajora")
	assert.NoError(t, err, "failed to create profile")

	_, _, err = mdStore.CreateProfile(name, "autumnmajora")
	assert.EqualError(t, err, ErrProfileNameNotAvailable.Error())
}

func TestDeleteProfile(t *testing.T) {
	mdStore := createMetadataStore(t)

	name := "jbpratt"
	profile, _, err := mdStore.CreateProfile(name, "autumnmajora")
	assert.NoError(t, err, "failed to create profile")

	assert.NoError(t, mdStore.DeleteProfile(profile), "failed to delete profile")
}

func TestGetProfiles(t *testing.T) {
	mdStore := createMetadataStore(t)

	_, _, err := mdStore.CreateProfile("jbpratt", "autumnmajora")
	assert.NoError(t, err, "failed to create profile")
	_, _, err = mdStore.CreateProfile("autumn", "jbprattmajora")
	assert.NoError(t, err, "failed to create profile")
	_, _, err = mdStore.CreateProfile("majora", "jbprattautumn")
	assert.NoError(t, err, "failed to create profile")

	summaries, err := mdStore.GetProfiles()
	assert.NoError(t, err, "failed to get profile summaries")

	assert.Equal(t, 3, len(summaries))
}
