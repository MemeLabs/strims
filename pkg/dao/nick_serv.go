package dao

import (
	"strconv"

	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

const nickRecordPrefix = "nickRecord:"

func prefixNickRecord(id uint64) string {
	return nickRecordPrefix + strconv.FormatUint(id, 10)
}

// UpsertNickRecord inserts or updates the record in the provided kv store
func UpsertNickRecord(s kv.RWStore, v *pb.NickServRecord) error {
	return s.Update(func(tx kv.RWTx) error {
		return tx.Put(prefixNickRecord(v.Id), v)
	})
}

// DeleteNickRecord deletes the record from the provided kv store
func DeleteNickRecord(s kv.RWStore, id uint64) error {
	return s.Update(func(tx kv.RWTx) error {
		return tx.Delete(prefixNickRecord(id))
	})
}

// GetNickRecord retrieves the record with the specified id from the provided kv store
func GetNickRecord(s kv.Store, id uint64) (v *pb.NickServRecord, err error) {
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(prefixNickRecord(id), v)
	})
	return v, err
}

// GetAllNickRecords gets all records from the provided kv store
func GetAllNickRecords(s kv.Store) (v []*pb.NickServRecord, err error) {
	err = s.View(func(tx kv.Tx) error {
		return tx.ScanPrefix(nickRecordPrefix, v)
	})
	return v, err
}
