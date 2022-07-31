// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package kv

import (
	"errors"

	"google.golang.org/protobuf/proto"
)

// ErrRecordNotFound ...
var ErrRecordNotFound = errors.New("record not found")

// BlobStore ...
type BlobStore interface {
	Close() error
	CreateStoreIfNotExists(table string) error
	DeleteStore(table string) error
	View(table string, fn func(tx BlobTx) error) error
	Update(table string, fn func(tx BlobTx) error) error
}

// BlobTx ...
type BlobTx interface {
	Put(key string, value []byte) error
	Get(key string) ([]byte, error)
	Delete(key string) error
	ScanPrefix(prefix string) ([][]byte, error)
	ScanCursor(cursor Cursor) ([][]byte, error)
}

// Store ...
type Store interface {
	View(fn func(tx Tx) error) error
}

// RWStore ...
type RWStore interface {
	Store
	Update(fn func(tx RWTx) error) error
}

// Tx ..
type Tx interface {
	Store
	Get(key string, m proto.Message) error
	ScanPrefix(prefix string, messages any) error
	ScanCursor(cursor Cursor, messages any) error
}

// RWTx ...
type RWTx interface {
	RWStore
	Delete(key string) error
	Get(key string, m proto.Message) error
	Put(key string, m proto.Message) error
	ScanPrefix(prefix string, messages any) error
	ScanCursor(cursor Cursor, messages any) error
}

type Cursor struct {
	Prefix string
	After  string
	Before string
	First  int
	Last   int
}
