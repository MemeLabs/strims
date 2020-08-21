package service

import (
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/petar/GoLLRB/llrb"
	"github.com/stretchr/testify/assert"
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

func TestStore(t *testing.T) {
	store := NickServStore{
		records: llrb.New(),
		nicks:   make(map[string]*nickServItem),
	}

	key := []byte{0xBE, 0xEF}

	record := &pb.NickServRecord{
		Id:   1,
		Nick: "bob",
		Key:  key,
	}

	// insert record
	store.Insert(record)

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
	assert.Equal(t, newRecord, r2)
	assert.Equal(t, newRecord, r)
}
