// +build js

package wasmio

import (
	"errors"
	"syscall/js"

	"github.com/MemeLabs/go-ppspp/pkg/dao"
)

// NewKVStore ...
func NewKVStore(bridge js.Value) *KVStore {
	return &KVStore{
		bridge: bridge,
	}
}

// KVStore ...
type KVStore struct {
	bridge js.Value
}

// CreateStoreIfNotExists ...
func (s *KVStore) CreateStoreIfNotExists(table string) error {
	return s.transact(table, true, true, func(tx dao.Tx) error { return nil })
}

// DeleteStore ...
func (s *KVStore) DeleteStore(table string) error {
	return callProxy(s.bridge, "deleteKVStore", []interface{}{table})
}

// View ...
func (s *KVStore) View(table string, fn func(tx dao.Tx) error) error {
	return s.transact(table, false, true, fn)
}

// Update ...
func (s *KVStore) Update(table string, fn func(tx dao.Tx) error) error {
	return s.transact(table, false, false, fn)
}

func (s *KVStore) transact(table string, createTable, readOnly bool, fn func(tx dao.Tx) error) error {
	tx := &KVTx{
		proxy: s.bridge.Call("openKVStore", table, createTable, readOnly),
	}
	err := fn(tx)
	if err != nil {
		tx.Rollback()
		return err
	}
	return tx.Commit()
}

// KVTx ...
type KVTx struct {
	proxy js.Value
}

// Put ...
func (t *KVTx) Put(key string, value []byte) error {
	b := jsUint8Array.New(len(value))
	js.CopyBytesToJS(b, value)

	return callProxy(t.proxy, "put", []interface{}{key, b})
}

// Delete ...
func (t *KVTx) Delete(key string) error {
	return callProxy(t.proxy, "delete", []interface{}{key})
}

// Get ...
func (t *KVTx) Get(key string) (value []byte, err error) {
	readValue := func(arg js.Value) error {
		if arg.IsUndefined() {
			return dao.ErrRecordNotFound
		}
		value = make([]byte, arg.Get("length").Int())
		js.CopyBytesToGo(value, arg)
		return nil
	}
	err = callProxy(t.proxy, "get", []interface{}{key}, readValue)
	return
}

// ScanPrefix ...
func (t *KVTx) ScanPrefix(prefix string) (values [][]byte, err error) {
	readValue := func(arg js.Value) error {
		l := arg.Get("length").Int()
		values = make([][]byte, l)
		for i := 0; i < l; i++ {
			value := arg.Index(i)
			values[i] = make([]byte, value.Get("length").Int())
			js.CopyBytesToGo(values[i], value)
		}
		return nil
	}
	err = callProxy(t.proxy, "scanPrefix", []interface{}{prefix}, readValue)
	return
}

// Rollback ...
func (t *KVTx) Rollback() error {
	return callProxy(t.proxy, "rollback", []interface{}{})
}

// Commit ...
func (t *KVTx) Commit() error {
	return callProxy(t.proxy, "commit", []interface{}{})
}

func callProxy(proxy js.Value, method string, args []interface{}, valueReaders ...func(js.Value) error) error {
	done := make(chan error)
	callback := js.FuncOf(func(this js.Value, args []js.Value) interface{} {
		defer close(done)
		if !args[0].IsNull() {
			done <- errors.New(args[0].String())
			return nil
		}

		for i := 0; i < len(valueReaders) && i+1 < len(args); i++ {
			if err := valueReaders[i](args[i+1]); err != nil {
				done <- err
			}
		}
		return nil
	})
	defer callback.Release()
	proxy.Call(method, append(args, callback)...)
	return <-done
}
