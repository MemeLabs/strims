package service

import (
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/memkv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/stretchr/testify/assert"
)

func createProfileStore(t *testing.T) (*pb.Profile, *dao.ProfileStore) {
	t.Helper()

	profile, err := dao.NewProfile("jbpratt")
	assert.Nil(t, err, "failed to create profile")

	key, err := dao.NewStorageKey("majoraautumn")
	assert.Nil(t, err, "failed to storage key")

	kvStore, err := memkv.NewStore("strims")
	assert.Nil(t, err, "failed to kv store")

	pfStore := dao.NewProfileStore(1, kvStore, key)
	assert.Nil(t, pfStore.Init(profile), "failed to create profile store")

	return profile, pfStore
}

func TestMarshalAndUnmarshalSessionID(t *testing.T) {
	sess := newSession()
	sess.Init(createProfileStore(t))

	pid, _, err := UnmarshalSessionID(sess.ID())
	assert.Nil(t, err, "failed to unmarshal session ID")
	assert.Equal(t, sess.profile.GetId(), pid)
}
