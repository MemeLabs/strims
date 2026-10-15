// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

// Generates golden vectors for crates/strims-crypto.
//
// Run from the repo root:
//
//	go run -tags golden ./crates/strims-crypto/golden
//
// Each line of vectors.hex is "<name> <hex> [<hex> ...]"; "=" is an empty
// value. Certificates come from the real internal/dao functions, so serials
// and validity times differ on each run; "now" records the generation time.
package main

import (
	"bufio"
	"crypto/ed25519"
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/MemeLabs/strims/internal/dao"
	chatv1 "github.com/MemeLabs/strims/pkg/apis/chat/v1"
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
	"github.com/MemeLabs/strims/pkg/apis/type/image"
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"github.com/MemeLabs/strims/pkg/apis/type/swarm"
	vpnv1 "github.com/MemeLabs/strims/pkg/apis/vpn/v1"
	"github.com/MemeLabs/strims/pkg/ed25519util"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"golang.org/x/crypto/curve25519"
	"google.golang.org/protobuf/proto"

	_ "unsafe" // go:linkname
)

// Unexported internal/dao functions, linked so the vectors come from the
// real implementation.
//
//go:linkname serializeCertificate github.com/MemeLabs/strims/internal/dao.serializeCertificate
func serializeCertificate(cert *certificate.Certificate) ([]byte, int)

//go:linkname serializeCertificateRequest github.com/MemeLabs/strims/internal/dao.serializeCertificateRequest
func serializeCertificateRequest(csr *certificate.CertificateRequest) ([]byte, int)

//go:linkname marshalSignableMessage github.com/MemeLabs/strims/internal/dao.marshalSignableMessage
func marshalSignableMessage(m dao.SignableMessage) *[]byte

const certTTL = time.Hour * 24 * 365 * 2

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

func marshal(m proto.Message) []byte {
	return must(proto.MarshalOptions{Deterministic: true}.Marshal(m))
}

func u32(v uint32) []byte { return binary.BigEndian.AppendUint32(nil, v) }

func seq(start byte, n int) []byte {
	b := make([]byte, n)
	for i := range b {
		b[i] = start + byte(i)
	}
	return b
}

func newKey(i int) *key.Key {
	seed := seq(byte(i*32), ed25519.SeedSize)
	priv := ed25519.NewKeyFromSeed(seed)
	k := &key.Key{Type: key.KeyType_KEY_TYPE_ED25519, Private: priv, Public: priv.Public().(ed25519.PublicKey)}
	put(fmt.Sprintf("key_%d", i), seed, marshal(k))
	return k
}

func certs(keys []*key.Key) {
	now := timeutil.Now()
	put("now", binary.BigEndian.AppendUint64(nil, uint64(now)))

	networkKey, memberKey, leafKey := keys[0], keys[1], keys[2]

	root := must(dao.NewSelfSignedCertificate(networkKey, certificate.KeyUsage_KEY_USAGE_SIGN, certTTL, dao.WithSubject("network")))

	memberCSR := must(dao.NewCertificateRequest(
		memberKey,
		certificate.KeyUsage_KEY_USAGE_PEER|certificate.KeyUsage_KEY_USAGE_SIGN,
		dao.WithSubject("alias"),
	))
	if err := dao.VerifyCertificateRequest(memberCSR, certificate.KeyUsage_KEY_USAGE_PEER|certificate.KeyUsage_KEY_USAGE_SIGN); err != nil {
		panic(err)
	}
	b, _ := serializeCertificateRequest(memberCSR)
	put("csr_member", marshal(memberCSR), b)

	member := must(dao.SignCertificateRequest(memberCSR, certTTL, networkKey))
	member.ParentOneof = &certificate.Certificate_Parent{Parent: root}

	leafCSR := must(dao.NewCertificateRequest(leafKey, certificate.KeyUsage_KEY_USAGE_PEER|certificate.KeyUsage_KEY_USAGE_BOOTSTRAP))
	b, _ = serializeCertificateRequest(leafCSR)
	put("csr_leaf", marshal(leafCSR), b)

	leaf := must(dao.SignCertificateRequest(leafCSR, 24*time.Hour, memberKey))
	leaf.ParentOneof = &certificate.Certificate_Parent{Parent: member}

	if err := dao.VerifyCertificate(leaf); err != nil {
		panic(err)
	}

	put("cert_chain", marshal(leaf))
	for _, c := range []struct {
		name string
		cert *certificate.Certificate
	}{{"root", root}, {"member", member}, {"leaf", leaf}} {
		b, _ := serializeCertificate(c.cert)
		put("cert_serialized_"+c.name, b)
	}
	put("cert_network_key", dao.CertificateNetworkKey(leaf))
}

func signable(keys []*key.Key) {
	htr := &vpnv1.HashTableMessage_Record{
		Salt:      []byte("salt"),
		Value:     seq(0x40, 100),
		Timestamp: 1_700_000_000_000,
	}
	pir := &vpnv1.PeerIndexMessage_Record{
		Hash:      seq(0x01, 32),
		HostId:    keys[1].Public,
		Port:      1234,
		Timestamp: -77,
	}
	snippet := &networkv1directory.ListingSnippet{
		Title:       "title",
		Description: "description",
		Tags:        []string{"one", "two"},
		Category:    "category",
		ChannelName: "channel",
		UserCount:   12345,
		Live:        true,
		Thumbnail:   &networkv1directory.ListingSnippetImage{SourceOneof: &networkv1directory.ListingSnippetImage_Image{Image: &image.Image{Type: image.ImageType_IMAGE_TYPE_JPEG, Height: 720, Width: 1280, Data: seq(0xf0, 16)}}},
		ChannelLogo: &networkv1directory.ListingSnippetImage{SourceOneof: &networkv1directory.ListingSnippetImage_Url{Url: "https://example.com/logo.png"}},
		VideoHeight: 720,
		VideoWidth:  1280,
		ThemeColor:  0xff00ff00,
		StartTime:   1_700_000_000,
	}
	for _, m := range []struct {
		name string
		key  *key.Key
		msg  dao.SignableMessage
	}{
		{"hash_table_record", keys[3], htr},
		{"peer_index_record", keys[1], pir},
		{"listing_snippet", keys[2], snippet},
	} {
		if err := dao.SignMessage(m.msg, m.key); err != nil {
			panic(err)
		}
		if err := dao.VerifyMessage(m.msg); err != nil {
			panic(err)
		}
		put("signable_"+m.name, marshal(m.msg), *marshalSignableMessage(m.msg))
	}
}

func crc32s() {
	for _, m := range []struct {
		name string
		msg  proto.Message
	}{
		{"swarm_cache", &swarm.Cache{
			Id:  42,
			Uri: "magnet:?xt=urn:ppspp:abcd",
			Integrity: &swarm.Cache_Integrity{
				MerkleIntegrity: &swarm.Cache_MerkleIntegrity{Timestamps: []int64{3, -4}, Signatures: [][]byte{seq(0x01, 64), seq(0x02, 64)}},
			},
			Data:  seq(0x10, 3000),
			Epoch: &swarm.Cache_Epoch{Timestamp: 1_700_000_000_000, Signature: seq(0x05, 64)},
		}},
		{"chat_emote", &chatv1.Emote{
			Id:     1,
			Name:   "PEPE",
			Images: []*chatv1.EmoteImage{{Data: seq(0x00, 10), FileType: chatv1.EmoteFileType_FILE_TYPE_PNG, Height: 28, Width: 28}},
			Effects: []*chatv1.EmoteEffect{
				{Effect: &chatv1.EmoteEffect_SpriteAnimation_{SpriteAnimation: &chatv1.EmoteEffect_SpriteAnimation{FrameCount: 4, DurationMs: 400}}},
			},
			Labels: []string{"a"},
			Enable: true,
		}},
		{"chat_modifier", &chatv1.Modifier{Id: 3, Name: "spin", Priority: 10, ProcChance: 0.5}},
		{"chat_tag", &chatv1.Tag{Id: 4, Name: "nsfw", Color: "#f00", Sensitive: true}},
		{"chat_server", &chatv1.Server{
			Id:            5,
			NetworkKey:    seq(0x20, 32),
			Key:           &key.Key{Type: key.KeyType_KEY_TYPE_ED25519, Private: seq(0, 64), Public: seq(0x40, 32)},
			Room:          &chatv1.Room{Name: "room", Css: "a{}"},
			AdminPeerKeys: [][]byte{seq(1, 32), seq(2, 32)},
		}},
		{"chat_server_icon", &chatv1.ServerIcon{Id: 6, Image: &image.Image{Type: image.ImageType_IMAGE_TYPE_PNG, Data: seq(0x89, 20)}}},
		{"empty_cache", &swarm.Cache{}},
	} {
		put("crc32_"+m.name, marshal(m.msg), u32(dao.CRC32Message(m.msg)))
	}
}

// p = 2^255 - 19, little-endian, plus small offsets.
func pPlus(d int) []byte {
	b := make([]byte, 32)
	for i := range b {
		b[i] = 0xff
	}
	b[31] = 0x7f
	b[0] = byte(0xed + d)
	return b
}

func small(v byte) []byte {
	b := make([]byte, 32)
	b[0] = v
	return b
}

func curves(keys []*key.Key) {
	for i, k := range keys {
		put(fmt.Sprintf("curve_key_%d", i), marshal(ed25519util.KeyToCurve25519(k)))
	}

	highBit := append([]byte(nil), keys[0].Public...)
	highBit[31] ^= 0x80
	h := sha256.Sum256([]byte("not a curve point"))
	inputs := [][]byte{
		small(0), small(1), small(2), small(9),
		pPlus(-1), pPlus(0), pPlus(1), pPlus(18),
		highBit, h[:], keys[1].Public, keys[2].Public,
	}
	for i, in := range inputs {
		var pub, out [32]byte
		copy(pub[:], in)
		ed25519util.PublicKeyToCurve25519(&out, &pub)
		put(fmt.Sprintf("pub2curve_%d", i), in, out[:])
	}

	pairs := [][2]int{{0, 1}, {1, 0}, {2, 3}, {3, 3}}
	for _, p := range pairs {
		a, b := ed25519util.KeyToCurve25519(keys[p[0]]), ed25519util.KeyToCurve25519(keys[p[1]])
		// vnic.handshakeAESLink
		put(fmt.Sprintf("aes_link_%d_%d", p[0], p[1]), must(curve25519.X25519(a.Private, b.Public)))
	}

	for _, p := range pairs {
		// vpn.newMessageCipher
		var ed25519Private [64]byte
		var ed25519Public [32]byte
		copy(ed25519Public[:], keys[p[1]].Public)
		copy(ed25519Private[:], keys[p[0]].Private)
		var curve25519Private, curve25519Public [32]byte
		ed25519util.PrivateKeyToCurve25519(&curve25519Private, &ed25519Private)
		ed25519util.PublicKeyToCurve25519(&curve25519Public, &ed25519Public)
		put(fmt.Sprintf("message_cipher_%d_%d", p[0], p[1]), must(curve25519.X25519(curve25519Private[:], curve25519Public[:])))
	}

	scalar := seq(0x77, 32)
	for i, point := range [][]byte{small(0), small(1), small(9), pPlus(-1), pPlus(0), pPlus(1), pPlus(18), h[:]} {
		out, err := curve25519.X25519(scalar, point)
		if err != nil {
			put(fmt.Sprintf("x25519_%d", i), scalar, point, []byte("error"))
		} else {
			put(fmt.Sprintf("x25519_%d", i), scalar, point, out)
		}
	}
}

func main() {
	f := must(os.Create(filepath.Join("crates", "strims-crypto", "golden", "vectors.hex")))
	defer f.Close()
	w = bufio.NewWriter(f)
	defer w.Flush()

	var keys []*key.Key
	for i := 0; i < 4; i++ {
		keys = append(keys, newKey(i))
	}

	for i, k := range keys[:2] {
		for j, msg := range [][]byte{{}, []byte("hello"), seq(0, 255), seq(1, 1000)} {
			put(fmt.Sprintf("sign_%d_%d", i, j), msg, ed25519.Sign(k.Private, msg))
		}
	}

	certs(keys)
	signable(keys)
	crc32s()
	curves(keys)
}
