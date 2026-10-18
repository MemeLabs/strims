// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

// Generates golden vectors for crates/strims-dao.
//
// Run from the repo root:
//
//	go run -tags golden ./crates/strims-dao/golden
//
// Each line of vectors.hex and store.hex is "<name> <hex> [<hex> ...]";
// "=" is an empty value. Sealed values, keys and ids are random, so they
// differ on each run. store.hex holds a profile store written through the
// dao API: its kv contents, its records as the dao reads them, and a
// compressed replication delta.
//
// With -verify <file>, checks a store another implementation wrote, as
// the file describes it, and prints "ok".
package main

import (
	"bufio"
	"bytes"
	"crypto/ed25519"
	"encoding/base32"
	"encoding/base64"
	"encoding/hex"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/MemeLabs/strims/internal/dao"
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	"github.com/MemeLabs/strims/pkg/apis/type/key"

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

func mediaListing(uri string) *networkv1directory.Listing {
	return &networkv1directory.Listing{Content: &networkv1directory.Listing_Media_{
		Media: &networkv1directory.Listing_Media{MimeType: "video/mp4", SwarmUri: uri},
	}}
}

// directoryListingKeys writes "dir_listing_key_<i> <network id> <listing>
// <key>": the key the directory's listing index holds for a listing.
func directoryListingKeys() {
	id := seq(0xa0, 32)
	unpadded := base32.StdEncoding.WithPadding(base32.NoPadding).EncodeToString(id)
	padded := base32.StdEncoding.EncodeToString(id)
	uri := func(xt string) string { return "magnet:?xt=" + xt + "&x.im=2&x.hf=4&x.sa=2&x.cs=1024&x.ps=64&x.sc=1" }
	urn := "urn:ppspp:"
	listings := []*networkv1directory.Listing{
		mediaListing(uri(urn + unpadded)),
		mediaListing(uri(urn + padded)),
		mediaListing(uri(urn + strings.ToLower(unpadded))),
		mediaListing(uri(urn + unpadded[:16] + strings.ToLower(unpadded[16:]))),
		mediaListing(uri(urn + unpadded[:20] + "%0D%0A" + unpadded[20:])),
		mediaListing(uri(urn + unpadded[:20] + "\r\n" + unpadded[20:])),
		mediaListing("magnet:?x.im=2&x.cs=1024"),
		mediaListing(uri(unpadded)),
		mediaListing(uri(urn+unpadded) + "#%zz"),
		mediaListing(uri(urn+unpadded) + "\x7f"),
		mediaListing(":?xt=" + urn + unpadded),
		mediaListing(""),
		mediaListing("MAGNET:?xt=urn%3Appspp%3A" + unpadded + "+"),
		mediaListing("magnet:?xt=a:b:MZXW6YQ%3D&xt=" + urn + unpadded),
		mediaListing("magnet:?xt;=1&xt=%zz&xt=" + urn + "MZXW6YTBOI======"),
		mediaListing("magnet:?xt=" + urn + "MY======AB"),
		mediaListing("magnet:?xt=" + urn + "MZXW6YTBMY======MY======"),
		mediaListing("magnet:?xt=" + urn + "MZX====="),
		mediaListing("magnet:?xt=" + urn + "MZXW6YTBMY======%FF"),
		{Content: &networkv1directory.Listing_Chat_{Chat: &networkv1directory.Listing_Chat{Key: seq(1, 32), Name: "chat"}}},
		{Content: &networkv1directory.Listing_Embed_{Embed: &networkv1directory.Listing_Embed{
			Service: networkv1directory.Listing_Embed_DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_STREAM,
			Id:      "streamer",
		}}},
		{Content: &networkv1directory.Listing_Embed_{Embed: &networkv1directory.Listing_Embed{
			Service: networkv1directory.Listing_Embed_Service(-2),
			Id:      "unknown",
		}}},
		{Content: &networkv1directory.Listing_Service_{Service: &networkv1directory.Listing_Service{
			Type:     "game",
			SwarmUri: uri(urn + unpadded),
		}}},
		{},
	}
	for i, l := range listings {
		networkID := 0x0102030405060708 + uint64(i)<<56
		put(fmt.Sprintf("dir_listing_key_%d", i), u64(networkID), marshal(l), dao.FormatDirectoryListingRecordListingKey(networkID, l))
	}
}

// chatProfilePeerKeys writes "chat_profile_peer_key_<i> <server id> <peer
// key> <key>": the key the chat profile index holds for a server's user.
func chatProfilePeerKeys() {
	for i, c := range []struct {
		server  uint64
		peerKey []byte
	}{
		{0, nil},
		{1, seq(0, 32)},
		{0x0102030405060708, []byte("peer")},
		{^uint64(0), seq(0xf0, 16)},
	} {
		put(fmt.Sprintf("chat_profile_peer_key_%d", i), u64(c.server), c.peerKey, dao.FormatChatProfilePeerKey(c.server, c.peerKey))
	}
}

// videoStreamKeys writes "video_stream_key_<i> <seed> <id> <token>
// <key>": the stream key of channel id with token, signed with the
// ed25519 key from seed; and "video_stream_key_parse_<i> <key> <id>
// <signature>": what the dao parses from key, with both values empty
// when it fails or panics.
func videoStreamKeys() {
	for i, c := range []struct {
		seed byte
		id   uint64
	}{
		{0, 0},
		{0x20, 1},
		{0x40, 127},
		{0x60, 128},
		{0x80, 0x0102030405060708},
		{0xa0, ^uint64(0)},
	} {
		private := ed25519.NewKeyFromSeed(seq(c.seed, ed25519.SeedSize))
		k := &key.Key{Type: key.KeyType_KEY_TYPE_ED25519, Private: private, Public: private.Public().(ed25519.PublicKey)}
		token := seq(c.seed+1, 32)
		put(fmt.Sprintf("video_stream_key_%d", i), seq(c.seed, ed25519.SeedSize), u64(c.id), token, []byte(dao.FormatVideoChannelStreamKey(c.id, token, k)))
	}

	sig := seq(0x40, ed25519.SignatureSize)
	raw := func(parts ...[]byte) string { return base64.RawURLEncoding.EncodeToString(bytes.Join(parts, nil)) }
	maxed := bytes.Repeat([]byte{0xff}, 9)
	for i, k := range []string{
		raw([]byte{0x05}, sig),
		raw([]byte{0x85, 0x00}, sig),
		raw(maxed, []byte{0x01}, sig),
		raw(maxed, []byte{0x02}, sig),
		raw(bytes.Repeat([]byte{0x80}, 10), []byte{0x01}, sig),
		raw(bytes.Repeat([]byte{0x80}, 10), []byte{0x01}, sig[:63]),
		raw(bytes.Repeat([]byte{0x80}, 5)),
		raw([]byte{0x05}, sig[:63]),
		raw([]byte{0x05}, sig, []byte{0x00}),
		raw([]byte{0x05}, sig) + "=",
		strings.NewReplacer("-", "+", "_", "/").Replace(raw([]byte{0x05}, bytes.Repeat([]byte{0xff}, 64))),
		"!!!!",
		"A",
		"",
	} {
		id, signature, ok := parseStreamKey(k)
		if !ok {
			put(fmt.Sprintf("video_stream_key_parse_%d", i), []byte(k), nil, nil)
			continue
		}
		put(fmt.Sprintf("video_stream_key_parse_%d", i), []byte(k), u64(id), signature)
	}
}

// parseStreamKey is the dao's parse of stream key k; ok is false when
// the parse fails or panics.
func parseStreamKey(k string) (id uint64, signature []byte, ok bool) {
	defer func() {
		if recover() != nil {
			ok = false
		}
	}()
	id, signature, err := dao.ParseVideoChannelStreamKey(k)
	return id, signature, err == nil
}

func main() {
	if len(os.Args) == 3 && os.Args[1] == "-verify" {
		verify(os.Args[2])
		return
	}

	path := filepath.Join("crates", "strims-dao", "golden", "vectors.hex")
	f := must(os.Create(path))
	defer f.Close()
	w = bufio.NewWriter(f)
	defer w.Flush()

	kdf()
	seal()
	indexHash()
	directoryListingKeys()
	chatProfilePeerKeys()
	videoStreamKeys()

	sf := must(os.Create(filepath.Join("crates", "strims-dao", "golden", "store.hex")))
	defer sf.Close()
	sw := bufio.NewWriter(sf)
	defer sw.Flush()
	storeScenario(sw)
}
