// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

// Generates golden vectors for crates/strims-ppspp.
//
// Run from the repo root:
//
//	go run -tags golden ./crates/strims-ppspp/golden > crates/strims-ppspp/golden/vectors.txt
//
// Each line is "<kind> key=value ..."; byte strings are hex, "-" is an
// empty or absent value. Chunk data is not stored: both sides generate it
// with dataByte from the absolute stream offset.
//
// ppspp.Writer reads the wall clock (timeutil.Now) for the epoch and for
// every signature, and the native timeutil has no hook for it. The writer
// scenarios therefore record the time Go used for each unit ("write ...
// ts=") and the Rust test replays the same writes with those times. Each
// write is at most one integrity unit long (a munro for merkle, a chunk for
// sign-all), so every Write call reads the clock at most once; the "none"
// scenario never reads it ("ts=*").
package main

import (
	"bufio"
	"bytes"
	"crypto/ed25519"
	"encoding/hex"
	"fmt"
	"os"
	"reflect"
	"sort"
	"strconv"
	"strings"
	"time"
	"unsafe"

	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"github.com/MemeLabs/strims/pkg/binmap"
	"github.com/MemeLabs/strims/pkg/bufioutil"
	"github.com/MemeLabs/strims/pkg/merkle"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/ppspp/codec"
	"github.com/MemeLabs/strims/pkg/ppspp/integrity"
	"github.com/MemeLabs/strims/pkg/ppspp/store"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"google.golang.org/protobuf/proto"
)

var out *bufio.Writer

func emit(kind string, kv ...string) {
	out.WriteString(kind)
	for _, s := range kv {
		out.WriteByte(' ')
		out.WriteString(s)
	}
	out.WriteByte('\n')
}

func hx(b []byte) string {
	if len(b) == 0 {
		return "-"
	}
	return hex.EncodeToString(b)
}

func kv(k string, v any) string {
	switch v := v.(type) {
	case []byte:
		return k + "=" + hx(v)
	default:
		return fmt.Sprintf("%s=%v", k, v)
	}
}

// dataByte is the stream content at absolute offset off. Mirrored in
// tests/golden.rs.
func dataByte(off uint64, salt uint64) byte {
	return byte(((off + salt) * 0x9E3779B97F4A7C15) >> 56)
}

func streamData(off, n uint64, salt uint64) []byte {
	b := make([]byte, n)
	for i := range b {
		b[i] = dataByte(off+uint64(i), salt)
	}
	return b
}

// encoder collects messages through the real codec.Writer.
type encoder struct {
	buf bytes.Buffer
	bw  *bufioutil.Writer
	cw  codec.Writer
}

func newEncoder() *encoder {
	e := &encoder{}
	e.bw = bufioutil.NewWriter(&e.buf, 1<<16)
	e.cw = codec.NewWriter(e.bw)
	return e
}

func (e *encoder) bytes() []byte {
	if err := e.cw.Flush(); err != nil {
		panic(err)
	}
	if err := e.bw.Flush(); err != nil {
		panic(err)
	}
	b := append([]byte(nil), e.buf.Bytes()...)
	e.buf.Reset()
	return b
}

func (e *encoder) WriteSignedIntegrity(m codec.SignedIntegrity) (int, error) {
	return e.cw.WriteSignedIntegrity(m)
}

func (e *encoder) WriteIntegrity(m codec.Integrity) (int, error) {
	return e.cw.WriteIntegrity(m)
}

func (e *encoder) WriteData(m codec.Data) (int, error) {
	return e.cw.WriteData(m)
}

func ms(n int64) timeutil.Time {
	return timeutil.New(n * int64(time.Millisecond))
}

func swarmID32() []byte {
	b := make([]byte, 32)
	for i := range b {
		b[i] = byte(0xa0 + i)
	}
	return b
}

func codecVectors() {
	sig64 := make([]byte, 64)
	for i := range sig64 {
		sig64[i] = byte(i * 3)
	}
	hash32 := make([]byte, 32)
	for i := range hash32 {
		hash32[i] = byte(0xff - i)
	}

	allOptions := codec.ProtocolOptions{
		codec.NewSwarmIdentifierProtocolOption(swarmID32()),
		&codec.VersionProtocolOption{Value: 2},
		&codec.MinimumVersionProtocolOption{Value: 2},
		&codec.LiveWindowProtocolOption{Value: 1 << 16},
		&codec.ChunkSizeProtocolOption{Value: 1024},
		&codec.ContentIntegrityProtectionMethodProtocolOption{Value: 2},
		&codec.MerkleHashTreeFunctionProtocolOption{Value: 4},
		&codec.LiveSignatureAlgorithmProtocolOption{Value: 2},
		&codec.ChunksPerSignatureProtocolOption{Value: 64},
		&codec.StreamCountProtocolOption{Value: 16},
		codec.NewEpochProtocolOption(ms(1_700_000_000_123), sig64),
	}

	msgs := []struct {
		name string
		m    codec.Message
	}{
		{"handshake_full", &codec.Handshake{ChannelID: 0x01020304, Options: allOptions}},
		{"handshake_empty", &codec.Handshake{ChannelID: 0, Options: codec.ProtocolOptions{}}},
		{"handshake_odd", &codec.Handshake{ChannelID: 0xffffffff, Options: codec.ProtocolOptions{
			&codec.StreamCountProtocolOption{Value: 0xffff},
			&codec.ChunkSizeProtocolOption{Value: 0xffffffff},
			codec.NewSwarmIdentifierProtocolOption(nil),
			&codec.VersionProtocolOption{Value: 0xff},
			&codec.VersionProtocolOption{Value: 1},
			codec.NewEpochProtocolOption(ms(-5), sig64),
		}}},
		{"restart", &codec.Restart{}},
		{"data", codec.NewData(16, 21, ms(1234), bytes.Repeat([]byte{0xff}, 32))},
		{"data_base", codec.NewData(16, 6, ms(1_700_000_000_000), streamData(0, 16, 1))},
		{"data_neg_time", codec.NewData(16, 0, ms(-1), streamData(0, 16, 2))},
		{"ack", &codec.Ack{Address: 22, DelaySample: codec.DelaySample{Duration: 1234 * time.Millisecond}}},
		{"ack_neg", &codec.Ack{Address: 1 << 40, DelaySample: codec.DelaySample{Duration: -3 * time.Millisecond}}},
		{"have", &codec.Have{Address: 1234}},
		{"have_big", &codec.Have{Address: codec.Address(binmap.All)}},
		{"have_none", &codec.Have{Address: codec.Address(binmap.None)}},
		{"integrity", &codec.Integrity{Address: 22, Hash: hash32}},
		{"signed_integrity", &codec.SignedIntegrity{Address: 15, Timestamp: codec.Timestamp{Time: ms(1_700_000_000_456)}, Signature: sig64}},
		{"request", &codec.Request{Address: 1234, Timestamp: codec.Timestamp{Time: ms(1234)}}},
		{"cancel", &codec.Cancel{Address: 1234}},
		{"choke", &codec.Choke{}},
		{"unchoke", &codec.Unchoke{}},
		{"ping", &codec.Ping{Nonce: codec.Nonce{Value: 1234}}},
		{"pong", &codec.Pong{Nonce: codec.Nonce{Value: 0xfedcba9876543210}, Delay: 77}},
		{"stream_request", &codec.StreamRequest{StreamAddress: codec.StreamAddress{Stream: 16, Address: 1234}}},
		{"stream_cancel", &codec.StreamCancel{Stream: 16}},
		{"stream_open", &codec.StreamOpen{StreamAddress: codec.StreamAddress{Stream: 0xffff, Address: 0}}},
		{"stream_close", &codec.StreamClose{Stream: 3}},
		{"end", &codec.End{}},
	}

	all := newEncoder()
	for _, c := range msgs {
		e := newEncoder()
		if _, err := e.cw.Write(c.m); err != nil {
			panic(err)
		}
		emit("msg", kv("name", c.name), kv("hex", e.bytes()))
		if c.name != "end" {
			if _, err := all.cw.Write(c.m); err != nil {
				panic(err)
			}
		}
	}
	all.cw.Write(&codec.End{})
	emit("payload", kv("name", "all"), kv("hex", all.bytes()))

	for _, h := range []codec.ChannelHeader{
		{Channel: 0, Length: 0},
		{Channel: 16, Length: 16384},
		{Channel: 1 << 35, Length: 0xffff},
	} {
		b := make([]byte, h.ByteLen())
		h.Marshal(b)
		emit("header", kv("channel", uint64(h.Channel)), kv("length", h.Length), kv("hex", b))
	}

	// A frame: two channel payloads behind their headers.
	frame := []byte{}
	for i, msgs := range [][]codec.Message{
		{&codec.Have{Address: 7}, &codec.Ping{Nonce: codec.Nonce{Value: 9}}},
		{&codec.Choke{}, &codec.Request{Address: 3, Timestamp: codec.Timestamp{Time: ms(5)}}},
	} {
		e := newEncoder()
		for _, m := range msgs {
			e.cw.Write(m)
		}
		p := e.bytes()
		h := codec.ChannelHeader{Channel: codec.Channel(i * 300), Length: uint16(len(p))}
		hb := make([]byte, h.ByteLen())
		h.Marshal(hb)
		frame = append(frame, hb...)
		frame = append(frame, p...)
	}
	emit("frame", kv("name", "two_channels"), kv("hex", frame))
}

var hashFuncs = []integrity.MerkleHashTreeFunction{
	integrity.MerkleHashTreeFunctionSHA1,
	integrity.MerkleHashTreeFunctionSHA256,
	integrity.MerkleHashTreeFunctionSHA512,
	integrity.MerkleHashTreeFunctionBLAKE2B256,
	integrity.MerkleHashTreeFunctionBLAKE2B512,
	integrity.MerkleHashTreeFunctionBLAKE3,
	integrity.MerkleHashTreeFunctionSHA3224,
	integrity.MerkleHashTreeFunctionSHA3256,
	integrity.MerkleHashTreeFunctionSHA3384,
	integrity.MerkleHashTreeFunctionSHA3512,
}

func merkleVectors() {
	const chunkSize = 32
	for _, f := range hashFuncs {
		for _, layer := range []uint64{0, 1, 2, 4} {
			root := binmap.NewBin(layer, 0)
			data := streamData(0, root.BaseLength()*chunkSize, uint64(f))
			t := merkle.NewTree(root, chunkSize, f.HashFunc())
			if _, err := t.Fill(root, data); err != nil {
				panic(err)
			}
			var digests []byte
			for b := root.BaseLeft(); b <= root.BaseRight(); b++ {
				digests = append(digests, t.Get(b)...)
			}
			emit("merkle", kv("fn", uint8(f)), kv("size", f.HashSize()), kv("cs", chunkSize), kv("root", uint64(root)), kv("digests", digests))
		}
	}
}

// unexported reads a private field of a ppspp.Swarm.
func unexported(s *ppspp.Swarm, name string) reflect.Value {
	v := reflect.ValueOf(s).Elem().FieldByName(name)
	return reflect.NewAt(v.Type(), unsafe.Pointer(v.UnsafeAddr())).Elem()
}

func swarmVerifier(s *ppspp.Swarm) integrity.SwarmVerifier {
	return unexported(s, "verifier").Interface().(integrity.SwarmVerifier)
}

func swarmStore(s *ppspp.Swarm) *store.Buffer {
	return unexported(s, "store").Interface().(*store.Buffer)
}

func swarmEpoch(s *ppspp.Swarm) (timeutil.Time, []byte) {
	e := unexported(s, "epoch")
	ts := timeutil.Time(e.FieldByName("Timestamp").Int())
	sig := e.FieldByName("Signature").Bytes()
	return ts, sig
}

type writerScenario struct {
	name      string
	pm        integrity.ProtectionMethod
	hf        integrity.MerkleHashTreeFunction
	sa        integrity.LiveSignatureAlgorithm
	cs, cps   int
	lw        int
	streams   int
	seed      byte
	salt      uint64
	phase1    []int
	phase2    []int
	haveCases [][2][]binmap.Bin // {bins to verify, peer have bins}
	dataBins  []binmap.Bin
}

func (sc *writerScenario) unit() int {
	if sc.pm == integrity.ProtectionMethodMerkleTree {
		return sc.cs * sc.cps
	}
	return sc.cs
}

func repeatSizes(total, maxSize int, pattern []int) []int {
	var out []int
	for i := 0; total > 0; i++ {
		n := pattern[i%len(pattern)]
		if n > maxSize {
			n = maxSize
		}
		if n > total {
			n = total
		}
		out = append(out, n)
		total -= n
	}
	return out
}

func runWriter(sc writerScenario) {
	seed := bytes.Repeat([]byte{sc.seed}, ed25519.SeedSize)
	priv := ed25519.NewKeyFromSeed(seed)
	k := &key.Key{Type: key.KeyType_KEY_TYPE_ED25519, Private: priv, Public: priv.Public().(ed25519.PublicKey)}

	opt := ppspp.SwarmOptions{
		ChunkSize:          sc.cs,
		ChunksPerSignature: sc.cps,
		StreamCount:        sc.streams,
		LiveWindow:         sc.lw,
		Integrity: integrity.VerifierOptions{
			ProtectionMethod:       sc.pm,
			MerkleHashTreeFunction: sc.hf,
			LiveSignatureAlgorithm: sc.sa,
		},
	}
	w, err := ppspp.NewWriter(ppspp.WriterOptions{SwarmOptions: opt, Key: k})
	if err != nil {
		panic(err)
	}
	s := w.Swarm()
	v := swarmVerifier(s)
	buf := swarmStore(s)

	emit("writer", kv("name", sc.name), kv("pm", uint8(sc.pm)), kv("hf", uint8(sc.hf)), kv("sa", uint8(sc.sa)),
		kv("cs", sc.cs), kv("cps", sc.cps), kv("lw", sc.lw), kv("sc", sc.streams), kv("seed", seed), kv("salt", sc.salt))
	ets, esig := swarmEpoch(s)
	emit("epoch", kv("ts", int64(ets)), kv("sig", esig))
	emit("uri", kv("str", s.URI().String()))

	var off uint64
	units := 0
	write := func(sizes []int) {
		for _, n := range sizes {
			if n > sc.unit() && sc.pm != integrity.ProtectionMethodNone {
				panic("write longer than one unit")
			}
			if _, err := w.Write(streamData(off, uint64(n), sc.salt)); err != nil {
				panic(err)
			}
			off += uint64(n)

			ts := "-"
			if u := int(off) / sc.unit(); u > units {
				units = u
				// the unit that just completed
				var b binmap.Bin
				if sc.pm == integrity.ProtectionMethodMerkleTree {
					b = binmap.NewBin(uint64(bitsLen(sc.cps)), uint64(u-1))
				} else {
					b = binmap.NewBin(0, uint64(u-1))
				}
				var e signedRecorder
				if sc.pm == integrity.ProtectionMethodNone {
					ts = "*" // the clock is not read
				} else if _, err := v.WriteIntegrity(b, binmap.New(), &e); err != nil {
					panic(err)
				} else {
					ts = strconv.FormatInt(int64(e.ts), 10)
				}
			}
			emit("write", kv("len", n), "ts="+ts)
		}
	}

	dump := func(phase string) {
		emit("phase", kv("name", phase), kv("tail", uint64(buf.Tail())), kv("next", uint64(buf.Next())))

		chunks := off / uint64(sc.cs)
		var bins []binmap.Bin
		for i := uint64(0); i < chunks; i++ {
			bins = append(bins, binmap.NewBin(0, i))
		}
		bins = append(bins, sc.dataBins...)
		for _, b := range bins {
			e := newEncoder()
			if _, err := buf.WriteData(b, ms(1_700_000_000_999), e); err != nil {
				emit("data", kv("bin", uint64(b)), kv("err", strings.ReplaceAll(err.Error(), " ", "_")))
				continue
			}
			emit("data", kv("bin", uint64(b)), kv("hex", e.bytes()))
		}

		cases := [][2][]binmap.Bin{{bins, nil}}
		cases = append(cases, sc.haveCases...)
		for _, c := range cases {
			have := binmap.New()
			var hs []string
			for _, b := range c[1] {
				have.Set(b)
				hs = append(hs, strconv.FormatUint(uint64(b), 10))
			}
			hd := "-"
			if len(hs) != 0 {
				hd = strings.Join(hs, ",")
			}
			for _, b := range c[0] {
				e := newEncoder()
				if _, err := v.WriteIntegrity(b, have, e); err != nil {
					emit("integrity", kv("bin", uint64(b)), "have="+hd, kv("err", strings.ReplaceAll(err.Error(), " ", "_")))
					continue
				}
				emit("integrity", kv("bin", uint64(b)), "have="+hd, kv("hex", e.bytes()))
			}
		}

		c, err := s.ExportCache()
		if err != nil {
			emit("cache", kv("err", strings.ReplaceAll(err.Error(), " ", "_")))
		} else {
			b, err := proto.MarshalOptions{Deterministic: true}.Marshal(c)
			if err != nil {
				panic(err)
			}
			emit("cache", kv("hex", b))
		}
	}

	write(sc.phase1)
	dump("one")
	if len(sc.phase2) != 0 {
		write(sc.phase2)
		dump("two")
	}
	emit("end")
}

func bitsLen(n int) int {
	l := 0
	for n > 1 {
		n >>= 1
		l++
	}
	return l
}

type signedRecorder struct {
	ts timeutil.Time
}

func (r *signedRecorder) WriteSignedIntegrity(m codec.SignedIntegrity) (int, error) {
	r.ts = m.Timestamp.Time
	return 0, nil
}

func (r *signedRecorder) WriteIntegrity(m codec.Integrity) (int, error) {
	return 0, nil
}

func writerVectors() {
	pattern := []int{100, 412, 512, 37, 475, 300, 212, 1, 511}
	runWriter(writerScenario{
		name: "merkle_blake2b_ed25519",
		pm:   integrity.ProtectionMethodMerkleTree,
		hf:   integrity.MerkleHashTreeFunctionBLAKE2B256,
		sa:   integrity.LiveSignatureAlgorithmED25519,
		cs:   64, cps: 8, lw: 64, streams: 1,
		seed: 7, salt: 11,
		phase1: repeatSizes(5*512, 512, pattern),
		phase2: repeatSizes(12*512, 512, pattern),
		haveCases: [][2][]binmap.Bin{
			{{0, 2, 4, 6, 8, 14, 16, 30, 3, 7}, {4}},
			{{0, 2, 14, 16, 18}, {2}},
			{{0, 6, 10, 16}, {7}},
			{{0, 18, 20, 40, 1, 19}, {1, 9, 21}},
			{{0, 16, 32, 48, 64}, {3, 23, 32, 55}},
		},
		dataBins: []binmap.Bin{1, 3, 7, 15, 19, 23, 31, 47},
	})
	runWriter(writerScenario{
		name: "merkle_sha256_nosig",
		pm:   integrity.ProtectionMethodMerkleTree,
		hf:   integrity.MerkleHashTreeFunctionSHA256,
		sa:   integrity.LiveSignatureAlgorithmNone,
		cs:   32, cps: 4, lw: 32, streams: 2,
		seed: 9, salt: 3,
		phase1: repeatSizes(6*128, 128, []int{128, 50, 78, 100, 28}),
		phase2: repeatSizes(9*128, 128, []int{128, 50, 78, 100, 28}),
		haveCases: [][2][]binmap.Bin{
			{{0, 2, 4, 6, 8, 9, 3}, {2, 5}},
		},
		dataBins: []binmap.Bin{1, 3, 5, 7, 11},
	})
	for _, hf := range []integrity.MerkleHashTreeFunction{
		integrity.MerkleHashTreeFunctionSHA1,
		integrity.MerkleHashTreeFunctionSHA512,
		integrity.MerkleHashTreeFunctionBLAKE2B512,
		integrity.MerkleHashTreeFunctionBLAKE3,
		integrity.MerkleHashTreeFunctionSHA3224,
		integrity.MerkleHashTreeFunctionSHA3256,
		integrity.MerkleHashTreeFunctionSHA3384,
		integrity.MerkleHashTreeFunctionSHA3512,
	} {
		runWriter(writerScenario{
			name: fmt.Sprintf("merkle_fn%d", hf),
			pm:   integrity.ProtectionMethodMerkleTree,
			hf:   hf,
			sa:   integrity.LiveSignatureAlgorithmED25519,
			cs:   16, cps: 4, lw: 16, streams: 1,
			seed: byte(hf), salt: uint64(hf),
			phase1: repeatSizes(3*64, 64, []int{64, 10, 54}),
			haveCases: [][2][]binmap.Bin{
				{{0, 2, 4, 6, 8, 3}, {2}},
			},
		})
	}
	runWriter(writerScenario{
		name: "sign_all_ed25519",
		pm:   integrity.ProtectionMethodSignAll,
		sa:   integrity.LiveSignatureAlgorithmED25519,
		cs:   64, cps: 1, lw: 32, streams: 1,
		seed: 5, salt: 29,
		phase1: repeatSizes(20*64, 64, []int{64, 30, 34, 63, 1, 64}),
		phase2: repeatSizes(30*64, 64, []int{64, 30, 34, 63, 1, 64}),
		haveCases: [][2][]binmap.Bin{
			{{1, 3, 7, 5}, {0}},
		},
		dataBins: []binmap.Bin{1, 3, 5, 7, 9, 13},
	})
	runWriter(writerScenario{
		name: "none",
		pm:   integrity.ProtectionMethodNone,
		sa:   integrity.LiveSignatureAlgorithmNone,
		cs:   64, cps: 1, lw: 16, streams: 1,
		seed: 1, salt: 2,
		phase1:   repeatSizes(10*64, 300, []int{300, 40, 1, 299}),
		phase2:   repeatSizes(20*64, 300, []int{300, 40, 1, 299}),
		dataBins: []binmap.Bin{1, 3, 5, 7},
	})
}

func uriVectors() {
	id := swarmID32()
	emit("swarmid", kv("hex", id), kv("str", ppspp.SwarmID(id).String()))
	for _, b := range [][]byte{{}, {0}, {1, 2}, {1, 2, 3}, {0xff, 0xfe, 0xfd, 0xfc}, {1, 2, 3, 4, 5}, {9, 8, 7, 6, 5, 4}} {
		emit("swarmid", kv("hex", b), kv("str", ppspp.SwarmID(b).String()))
	}
	for _, s := range []string{
		"DVMYU5F5ZFJTL34ODXAUHS2FVBIK7TH2PNNRPODKJWZFTOGSR73Q",
		"AE",
		"AEB",   // dangling 1-char group: dropped
		"AEBAG", // 3 bytes
		"AEBAGBA",
		"AEBAGBAF",
		"AEBAGBAFA", // 9 chars: dangling 1-char group
		"AEB\nAGBA", // newlines are stripped
		"aeba",      // lower case is not in the alphabet
		"AE==",      // padding is not allowed
		"AE1",       // 1 is not in the alphabet
		"",
	} {
		id, err := ppspp.DecodeSwarmID(s)
		if err != nil {
			emit("decodeid", kv("str", []byte(s)), "err=1")
		} else {
			emit("decodeid", kv("str", []byte(s)), kv("hex", []byte(id)))
		}
	}

	opts := ppspp.NewDefaultSwarmOptions().URIOptions()
	emit("uristring", kv("str", ppspp.NewURI(id, opts).String()))
	emit("uristring", kv("str", ppspp.NewURI(id, ppspp.URIOptions{}).String()))
	emit("uristring", kv("str", ppspp.NewURI(nil, ppspp.URIOptions{codec.StreamCountOption: 32, codec.ChunkSizeOption: 1 << 31}).String()))

	good := ppspp.NewURI(id, opts).String()
	for _, s := range []string{
		good,
		strings.Replace(good, "magnet:", "MAGNET:", 1),
		strings.Replace(good, "magnet:", "MaGnEt:", 1),
		good + "&x.cs=2048",
		good + "&foo=bar&x.zz=1",
		strings.Replace(good, "x.cs=1024", "x.cs=%31024", 1),
		strings.Replace(good, "x.cs=1024", "x.cs=01024", 1),
		strings.Replace(good, "x.cs=1024", "x.cs=2147483647", 1),
		strings.Replace(good, "x.cs=1024", "x.cs=2147483648", 1),
		strings.Replace(good, "x.cs=1024", "x.cs=-1", 1),
		strings.Replace(good, "x.cs=1024", "x.cs=+1", 1),
		strings.Replace(good, "x.cs=1024", "x.cs=", 1),
		strings.Replace(good, "x.cs=1024", "x.cs", 1),
		strings.Replace(good, "x.cs=1024", "x.cs=1_0", 1),
		strings.Replace(good, "x.cs=1024", "x.cs=1%2", 1),
		strings.Replace(good, "x.cs=1024", "x.cs=1+2", 1),
		strings.Replace(good, "&x.cs=1024", "&x.cs=1024;x.ps=7", 1),
		good + "#frag",
		good + "&",
		"magnet:?x.cs=12&xt=urn:ppspp:AEBAGBAF",
		"magnet:?xt=urn:ppspp:AEBAGBAF&xt=urn:ppspp:AE",
		"magnet:?xt=urn%3Appspp%3AAEBAGBAF",
		"magnet:?xt=urn:ppspp:",
		"magnet:?xt=urn:other:AEBAGBAF",
		"magnet:?xt=urn:ppspp:aeba",
		"magnet:?xt=urn:ppspp:AEBAGBAFA",
		"magnet:?",
		"magnet:",
		"magnet://host/?xt=urn:ppspp:AE",
		"magnet:opaque?xt=urn:ppspp:AE",
		"http:?xt=urn:ppspp:AE",
		"?xt=urn:ppspp:AE",
		":?xt=urn:ppspp:AE",
		"1magnet:?xt=urn:ppspp:AE",
		"magnet:?xt=urn:ppspp:AE&x.sc=4&x.sc=8",
		"magnet:?xt=urn:ppspp:AE\x7f",
		"magnet:?xt=urn:ppspp:AE%zz",
	} {
		u, err := ppspp.ParseURI(s)
		if err != nil {
			emit("parseuri", kv("str", []byte(s)), "err=1")
			continue
		}
		var keys []int
		for k := range u.Options {
			keys = append(keys, int(k))
		}
		sort.Ints(keys)
		var parts []string
		for _, k := range keys {
			parts = append(parts, fmt.Sprintf("%d:%d", k, u.Options[codec.ProtocolOptionType(k)]))
		}
		o := "-"
		if len(parts) != 0 {
			o = strings.Join(parts, ",")
		}
		emit("parseuri", kv("str", []byte(s)), kv("id", []byte(u.ID)), "opts="+o)
	}
}

func main() {
	out = bufio.NewWriter(os.Stdout)
	defer out.Flush()

	emit("#", "generated by crates/strims-ppspp/golden/main.go; do not edit")
	codecVectors()
	merkleVectors()
	writerVectors()
	uriVectors()
}
