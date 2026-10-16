// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

// Generates golden vectors for crates/strims-dao.
//
// Run from the repo root:
//
//	go run -tags golden ./crates/strims-dao/golden
//
// Each line of vectors.hex is "<name> <hex> [<hex> ...]"; "=" is an empty
// value. Sealed values use random nonces, so they differ on each run.
package main

import (
	"bufio"
	"encoding/hex"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/MemeLabs/strims/internal/dao"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"

	_ "unsafe" // go:linkname
)

//go:linkname hashSecondaryIndexKey github.com/MemeLabs/strims/internal/dao.hashSecondaryIndexKey
func hashSecondaryIndexKey(key []byte, s any) string

type salter []byte

func (s salter) Salt() []byte { return s }

var w *bufio.Writer

func put(name string, values ...[]byte) {
	parts := []string{name}
	for _, v := range values {
		if len(v) == 0 {
			parts = append(parts, "=")
		} else {
			parts = append(parts, hex.EncodeToString(v))
		}
	}
	fmt.Fprintln(w, strings.Join(parts, " "))
}

func must[T any](v T, err error) T {
	if err != nil {
		panic(err)
	}
	return v
}

func seq(start byte, n int) []byte {
	b := make([]byte, n)
	for i := range b {
		b[i] = start + byte(i)
	}
	return b
}

func kdf() {
	cases := []struct {
		password   string
		salt       []byte
		iterations uint32
	}{
		{"", seq(0, 16), 1},
		{"hunter2", seq(16, 16), 1000},
		{"pässwörd 🔑", seq(32, 16), 100000},
		{"long " + strings.Repeat("x", 200), seq(48, 7), 3},
	}
	for i, c := range cases {
		record := &profilev1.StorageKey{
			KdfType: profilev1.KDFType_KDF_TYPE_PBKDF2_SHA256,
			KdfOptions: &profilev1.StorageKey_Pbkdf2Options{
				Pbkdf2Options: &profilev1.StorageKey_PBKDF2Options{
					Iterations: c.iterations,
					KeySize:    dao.KeySize,
					Salt:       c.salt,
				},
			},
		}
		k := must(dao.NewStorageKeyFromPassword(c.password, record))
		put(fmt.Sprintf("kdf_%d", i), []byte(c.password), must(dao.MarshalStorageKey(k)), k.Key())
	}
}

func seal() {
	key := seq(100, dao.KeySize)
	k := must(dao.NewStorageKeyFromBytes(key, nil))
	for i, n := range []int{0, 1, 16, 33, 1000} {
		p := seq(byte(i), n)
		put(fmt.Sprintf("seal_%d", i), key, p, must(k.Seal(p)))
	}
}

func indexHash() {
	for i, c := range []struct{ salt, key []byte }{
		{seq(0, 32), nil},
		{seq(0, 32), []byte("key")},
		{seq(32, 32), seq(0, 100)},
		{seq(0, 80), []byte("long salt")},
	} {
		put(fmt.Sprintf("index_hash_%d", i), c.salt, c.key, []byte(hashSecondaryIndexKey(c.key, salter(c.salt))))
	}
}

func main() {
	path := filepath.Join("crates", "strims-dao", "golden", "vectors.hex")
	f := must(os.Create(path))
	defer f.Close()
	w = bufio.NewWriter(f)
	defer w.Flush()

	kdf()
	seal()
	indexHash()
}
