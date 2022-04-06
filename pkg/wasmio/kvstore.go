//go:build js

package wasmio

import (
	"errors"
	"syscall/js"

	"github.com/MemeLabs/go-ppspp/pkg/kv"
)

// NewKVStore ...
func NewKVStore(bridge js.Value) kv.BlobStore {
	return &kvStore{
		bridge: bridge,
	}
}

// KVStore ...
type kvStore struct {
	bridge js.Value
}

// Close ...
func (s *kvStore) Close() error {
	return nil
}

// CreateStoreIfNotExists ...
func (s *kvStore) CreateStoreIfNotExists(table string) error {
	return s.transact(table, true, true, func(tx kv.BlobTx) error { return nil })
}

// DeleteStore ...
func (s *kvStore) DeleteStore(table string) error {
	return callProxy(s.bridge, "deleteKVStore", []any{table})
}

// View ...
func (s *kvStore) View(table string, fn func(tx kv.BlobTx) error) error {
	return s.transact(table, false, true, fn)
}

// Update ...
func (s *kvStore) Update(table string, fn func(tx kv.BlobTx) error) error {
	return s.transact(table, false, false, fn)
}

func (s *kvStore) transact(table string, createTable, readOnly bool, fn func(tx kv.BlobTx) error) error {
	tx := &kvTx{
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
type kvTx struct {
	proxy js.Value
}

// Put ...
func (t *kvTx) Put(key string, value []byte) error {
	b := jsUint8Array.New(len(value))
	js.CopyBytesToJS(b, value)

	return callProxy(t.proxy, "put", []any{key, b})
}

// Delete ...
func (t *kvTx) Delete(key string) error {
	return callProxy(t.proxy, "delete", []any{key})
}

// Get ...
func (t *kvTx) Get(key string) (value []byte, err error) {
	readValue := func(arg js.Value) error {
		if arg.IsUndefined() {
			return kv.ErrRecordNotFound
		}
		value = make([]byte, arg.Get("length").Int())
		js.CopyBytesToGo(value, arg)
		return nil
	}
	err = callProxy(t.proxy, "get", []any{key}, readValue)
	return
}

// ScanPrefix ...
func (t *kvTx) ScanPrefix(prefix string) (values [][]byte, err error) {
	return t.ScanCursor(kv.Cursor{After: prefix, Before: prefix})
}

// ScanCursor ...
func (t *kvTx) ScanCursor(cursor kv.Cursor) (values [][]byte, err error) {
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
	err = callProxy(t.proxy, "scanCursor", []any{cursor.After, cursor.Before, cursor.First, cursor.Last}, readValue)
	return
}

// Rollback ...
func (t *kvTx) Rollback() error {
	return callProxy(t.proxy, "rollback", []any{})
}

// Commit ...
func (t *kvTx) Commit() error {
	return callProxy(t.proxy, "commit", []any{})
}

func callProxy(proxy js.Value, method string, args []any, valueReaders ...func(js.Value) error) error {
	done := make(chan error)
	callback := js.FuncOf(func(this js.Value, args []js.Value) any {
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
