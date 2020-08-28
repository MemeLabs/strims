package service

import (
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/memkv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/petar/GoLLRB/llrb"
	"github.com/stretchr/testify/assert"
	"google.golang.org/protobuf/proto"
)

func TestSignVerifyNickServToken(t *testing.T) {

}

func TestDuplicateNameCreateRequestFails(t *testing.T) {

}

func TestAdminCanUpdateNicks(t *testing.T) {

}

func TestRequestFailsOnUnverifiedSignature(t *testing.T) {

}

func TestNameChangeQuota(t *testing.T) {

}

func TestBadRoleAssignment(t *testing.T) {

}

func createStore(t *testing.T) *dao.ProfileStore {
	// TODO: is this the right way to get a kv store for testing?
	t.Helper()

	profile, err := dao.NewProfile("jbpratt")
	assert.Nil(t, err, "failed to create profile")

	key, err := dao.NewStorageKey("majoraautumn")
	assert.Nil(t, err, "failed to storage key")

	kvStore, err := memkv.NewStore("strims")
	assert.Nil(t, err, "failed to kv store")

	pfStore := dao.NewProfileStore(1, kvStore, key)
	assert.Nil(t, pfStore.Init(profile), "failed to create profile store")

	return pfStore
}

func TestStore(t *testing.T) {
	store := NickServStore{
		records: llrb.New(),
		nicks:   make(map[string]*nickServItem),
		kv:      createStore(t),
	}

	key := []byte{0xBE, 0xEF}

	record := &pb.NickServRecord{
		Id:   1,
		Nick: "bob",
		Key:  key,
	}

	// insert record
	err := store.Insert(record)
	assert.NoError(t, err)

	// ensure that record was inserted correctly
	r := store.nicks["bob"].Record()
	assert.NotNil(t, r)
	assert.Equal(t, record, r)

	r2, err := store.Retrieve(key)
	assert.NoError(t, err)
	assert.Equal(t, record, r2)

	newRecord := &pb.NickServRecord{
		Id:   2,
		Nick: "brady",
		Key:  key,
	}

	// update record
	err = store.Update(newRecord, record.Nick)
	assert.NoError(t, err)

	// should the previously returned pointers point to newRecord?
	// this fails
	assert.True(t, proto.Equal(newRecord, r2))
	assert.True(t, proto.Equal(newRecord, r))
}
