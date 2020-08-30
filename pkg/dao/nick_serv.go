package dao

import (
	"strconv"

	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

const nickRecordPrefix = "nickservNick:"

func prefixNick(nickservID, recordID uint64) string {
	return prefixNickservInstance(nickservID) + strconv.FormatUint(recordID, 10)
}

func prefixNickservInstance(nickservID uint64) string {
	return nickRecordPrefix + strconv.FormatUint(nickservID, 10) + ":"
}

// UpsertNickRecord inserts or updates the record in the provided kv store
func UpsertNickRecord(s kv.RWStore, v *pb.NickservNick, nickservID uint64) error {
	return s.Update(func(tx kv.RWTx) error {
		return tx.Put(prefixNick(nickservID, v.Id), v)
	})
}

// DeleteNickRecord deletes the record from the provided kv store
func DeleteNickRecord(s kv.RWStore, nickservID, recordID uint64) error {
	return s.Update(func(tx kv.RWTx) error {
		return tx.Delete(prefixNick(nickservID, recordID))
	})
}

// GetNickRecord retrieves the record with the specified id from the provided kv store
func GetNickRecord(s kv.Store, nickservID, recordID uint64) (v *pb.NickservNick, err error) {
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(prefixNick(nickservID, recordID), v)
	})
	return v, err
}

// GetAllNickRecords gets all records from the provided kv store
func GetAllNickRecords(s kv.Store, nickservID uint64) (v []*pb.NickservNick, err error) {
	v = []*pb.NickservNick{}
	err = s.View(func(tx kv.Tx) error {
		return tx.ScanPrefix(prefixNickservInstance(nickservID), &v)
	})
	return v, err
}

func GetNickservConfig(s kv.Store, nickservID uint64) (v *pb.ServerConfig, err error) {
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(prefixNickservInstance(nickservID), v)
	})
	return v, err
}
