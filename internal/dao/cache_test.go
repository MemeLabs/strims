package dao

import (
	"testing"
	"time"

	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	"github.com/MemeLabs/go-ppspp/pkg/errutil"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/kv/kvtest"
	"github.com/stretchr/testify/assert"
)

func TestCacheInsertGet(t *testing.T) {
	blobStore := kvtest.NewMemStore()
	storageKey := errutil.Must(NewStorageKey("test"))
	profileStore := NewProfileStore(1111, storageKey, blobStore, nil)
	if err := profileStore.Init(); err != nil {
		panic(err)
	}

	c := NewChatProfileCache(profileStore, &CacheStoreOptions{
		TTL:        50 * time.Millisecond,
		GCInterval: 10 * time.Millisecond,
	})

	serverID := uint64(2222)
	peerKey := errutil.Must(GenerateKey()).Public

	profile, err := c.ByPeerKey.Get(FormatChatProfilePeerKey(serverID, peerKey))
	assert.Nil(t, profile)
	assert.ErrorIs(t, err, kv.ErrRecordNotFound)

	profile, found, err := c.ByPeerKey.GetOrInsert(
		FormatChatProfilePeerKey(serverID, peerKey),
		func() (*chatv1.Profile, error) {
			return NewChatProfile(profileStore, serverID, peerKey, "alias")
		},
	)
	assert.NotNil(t, profile)
	assert.False(t, found)
	assert.NoError(t, err)

	profile2, err := c.ByID.Get(profile.Id)
	if err != nil {
		panic(err)
	}
	assert.Equal(t, profile, profile2, "inserted record should be in cache")
	assert.NoError(t, err)

	time.Sleep(200 * time.Millisecond)

	profile3, err := c.ByID.Get(profile.Id)
	if err != nil {
		panic(err)
	}
	assert.NotEqual(t, profile, profile3, "record should have expired")
	assert.NoError(t, err)

	c.Close()
}
