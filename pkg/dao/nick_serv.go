package dao

import (
	"strconv"

	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

const nickRecordPrefix = "nickservNick:"

func prefixNickRecord(nickservID, recordID uint64) string {
	return prefix(nickservID) + strconv.FormatUint(recordID, 10)
}

func prefix(nickservID uint64) string {
	return nickRecordPrefix + strconv.FormatUint(nickservID, 10) + ":"
}

// UpsertNickRecord inserts or updates the record in the provided kv store
func UpsertNickRecord(s kv.RWStore, v *pb.NickServRecord, nickservID uint64) error {
	return s.Update(func(tx kv.RWTx) error {
		return tx.Put(prefixNickRecord(nickservID, v.Id), v)
	})
}

// DeleteNickRecord deletes the record from the provided kv store
func DeleteNickRecord(s kv.RWStore, nickservID, recordID uint64) error {
	return s.Update(func(tx kv.RWTx) error {
		return tx.Delete(prefixNickRecord(nickservID, recordID))
	})
}

// GetNickRecord retrieves the record with the specified id from the provided kv store
func GetNickRecord(s kv.Store, nickservID, recordID uint64) (v *pb.NickServRecord, err error) {
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(prefixNickRecord(nickservID, recordID), v)
	})
	return v, err
}

// GetAllNickRecords gets all records from the provided kv store
func GetAllNickRecords(s kv.Store, nickservID uint64) (v []*pb.NickServRecord, err error) {
	v = []*pb.NickServRecord{}
	err = s.View(func(tx kv.Tx) error {
		return tx.ScanPrefix(prefix(nickservID), &v)
	})
	return v, err
}

func GetNickservConfig(s kv.Store, nickservID uint64) (v *pb.ServerConfig, err error) {
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(prefix(nickservID), v)
	})
	return v, err
}
