// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

// Generates golden vectors for crates/strims-chunkstream.
//
// Run from the repo root:
//
//	go run -tags golden ./crates/strims-chunkstream/golden
//
// Each line of vectors.hex is "<kind> <field> ..."; "=" is empty bytes.
//
//	frames <size> <record> <frames>
//	    one record through chunkstream.ZeroPadWriter, then Flush
//	frames_digest <size> <record len> <frames sha256> <headers>
//	    the same for large frames: the record is pattern(record len) and
//	    headers concatenates each frame's first 4 bytes
//	records <EventBroadcast> <stream>
//	    one EventBroadcast through protoutil.ChunkStreamWriter(1024)
//	stream <id> <size> <stream> <payload>,<payload>,...
//	    payloads through one protoutil.ChunkStreamWriter(size)
//	read <id> <offset> <end|err> <item>,<item>,...|-
//	    the payloads protoutil.ChunkStreamReader returns when its input
//	    starts at byte offset of stream id. An item is the index of one of
//	    that stream's payloads, or "x<hex>" for a payload that is none of
//	    them. "end": the reader ran out of input; "err": Read failed first.
//	    Offsets inside a frame header (1 to 3 past a frame start) are left
//	    out: the reader takes the 4 bytes there for a header.
//
// pattern(n) is byte i = (i*131 + 7) mod 256 for i < n.
package main

import (
	"bufio"
	"bytes"
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/strims/pkg/apis/type/image"
	"github.com/MemeLabs/strims/pkg/chunkstream"
	"github.com/MemeLabs/strims/pkg/protoutil"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/types/known/emptypb"
	"google.golang.org/protobuf/types/known/wrapperspb"
)

func check(err error) {
	if err != nil {
		panic(err)
	}
}

func pattern(n int) []byte {
	b := make([]byte, n)
	for i := range b {
		b[i] = byte(i*131 + 7)
	}
	return b
}

func h(b []byte) string {
	if len(b) == 0 {
		return "="
	}
	return hex.EncodeToString(b)
}

// buffer is the ioutil.WriteFlusher ChunkStreamWriter writes to.
type buffer struct{ bytes.Buffer }

func (b *buffer) Flush() error { return nil }

func zeroPad(size int, record []byte) []byte {
	var b bytes.Buffer
	w, err := chunkstream.NewZeroPadWriterSize(&b, size)
	check(err)
	_, err = w.Write(record)
	check(err)
	check(w.Flush())
	return b.Bytes()
}

func lengths(size int) []int {
	return []int{0, 1, size - 5, size - 4, size - 3, 2 * (size - 4), 5000}
}

func frames(w io.Writer) {
	for _, size := range []int{16, 32, 1024} {
		for _, n := range lengths(size) {
			r := pattern(n)
			fmt.Fprintf(w, "frames %d %s %s\n", size, h(r), h(zeroPad(size, r)))
		}
	}
	size := 128 * 1024
	for _, n := range lengths(size) {
		f := zeroPad(size, pattern(n))
		var headers []byte
		for i := 0; i < len(f); i += size {
			headers = append(headers, f[i:i+4]...)
		}
		sum := sha256.Sum256(f)
		fmt.Fprintf(w, "frames_digest %d %d %s %s\n", size, n, h(sum[:]), h(headers))
	}
}

func ping() *networkv1directory.EventBroadcast {
	return &networkv1directory.EventBroadcast{
		Events: []*networkv1directory.Event{
			{Body: &networkv1directory.Event_Ping_{Ping: &networkv1directory.Event_Ping{Time: 1257894000000000000}}},
		},
	}
}

func presence() *networkv1directory.EventBroadcast {
	return &networkv1directory.EventBroadcast{
		Events: []*networkv1directory.Event{
			{Body: &networkv1directory.Event_UserCountChange_{UserCountChange: &networkv1directory.Event_UserCountChange{
				Id: 7, UserCount: 3, RecentUserCount: 12,
			}}},
			{Body: &networkv1directory.Event_UserPresenceChange_{UserPresenceChange: &networkv1directory.Event_UserPresenceChange{
				Id: 9, Alias: "someone", PeerKey: pattern(32), Online: true, ListingIds: []uint64{7, 1 << 40},
			}}},
			{Body: &networkv1directory.Event_Unpublish_{Unpublish: &networkv1directory.Event_Unpublish{Id: 8}}},
		},
	}
}

func listing(thumbnail int) *networkv1directory.EventBroadcast {
	return &networkv1directory.EventBroadcast{
		Events: []*networkv1directory.Event{
			{Body: &networkv1directory.Event_ListingChange_{ListingChange: &networkv1directory.Event_ListingChange{
				Id: 7,
				Listing: &networkv1directory.Listing{Content: &networkv1directory.Listing_Chat_{
					Chat: &networkv1directory.Listing_Chat{Key: pattern(32), Name: "chat"},
				}},
				Snippet: &networkv1directory.ListingSnippet{
					Title: "title",
					Thumbnail: &networkv1directory.ListingSnippetImage{SourceOneof: &networkv1directory.ListingSnippetImage_Image{
						Image: &image.Image{Type: image.ImageType_IMAGE_TYPE_JPEG, Data: pattern(thumbnail)},
					}},
				},
			}}},
		},
	}
}

// recordLen is the length of m's record: uvarint(size) || m.
func recordLen(m proto.Message) int {
	n := proto.Size(m)
	return binary.PutUvarint(make([]byte, binary.MaxVarintLen64), uint64(n)) + n
}

// filling returns a broadcast whose record exactly fills two 1024 byte frames.
func filling() *networkv1directory.EventBroadcast {
	for n := 1800; n < 2100; n++ {
		if m := listing(n); recordLen(m) == 2*(1024-4) {
			return m
		}
	}
	panic("no thumbnail size fills two frames")
}

func broadcasts() []*networkv1directory.EventBroadcast {
	return []*networkv1directory.EventBroadcast{ping(), presence(), listing(3000), filling()}
}

func writeStream(size int, msgs []proto.Message) []byte {
	var b buffer
	w, err := protoutil.NewChunkStreamWriter(&b, size)
	check(err)
	for _, m := range msgs {
		check(w.Write(m))
	}
	return b.Bytes()
}

func records(w io.Writer) {
	for _, m := range broadcasts() {
		b, err := proto.Marshal(m)
		check(err)
		fmt.Fprintf(w, "records %s %s\n", h(b), h(writeStream(1024, []proto.Message{m})))
	}
}

var errEnd = errors.New("end of input")

// input is the protoutil.OffsetReader a ChunkStreamReader reads from: the
// stream from offset on, then errEnd.
type input struct {
	b      []byte
	offset uint64
}

func (r *input) Read(p []byte) (int, error) {
	if len(r.b) == 0 {
		return 0, errEnd
	}
	n := copy(p, r.b)
	r.b = r.b[n:]
	return n, nil
}

func (r *input) Offset() uint64 { return r.offset }

func read(stream []byte, size, offset int) (payloads [][]byte, status string) {
	r := protoutil.NewChunkStreamReader(&input{b: stream[offset:], offset: uint64(offset)}, size)
	for {
		var m emptypb.Empty
		if err := r.Read(&m); err != nil {
			if errors.Is(err, errEnd) {
				return payloads, "end"
			}
			return payloads, "err"
		}
		payloads = append(payloads, m.ProtoReflect().GetUnknown())
	}
}

type stream struct {
	size     int
	payloads []proto.Message
	// offsets returns the start offsets to read from, given the stream length.
	offsets func(n int) []int
}

// everyOffset is every offset outside a frame header, end of input included.
func everyOffset(size int) func(n int) []int {
	return func(n int) (offsets []int) {
		for o := 0; o <= n; o++ {
			if p := o % size; p == 0 || p >= 4 {
				offsets = append(offsets, o)
			}
		}
		return offsets
	}
}

// someOffsets is each frame start and a few offsets inside each frame.
func someOffsets(size int) func(n int) []int {
	return func(n int) (offsets []int) {
		for f := 0; f <= n; f += size {
			for _, p := range []int{0, 4, 5, size / 2, size - 1} {
				if f+p <= n {
					offsets = append(offsets, f+p)
				}
			}
		}
		return offsets
	}
}

func bytesValues(lengths ...int) (msgs []proto.Message) {
	for i, n := range lengths {
		b := pattern(n)
		for j := range b {
			b[j] += byte(i)
		}
		msgs = append(msgs, wrapperspb.Bytes(b))
	}
	return msgs
}

func streams() []stream {
	var events []proto.Message
	for _, m := range broadcasts() {
		events = append(events, m)
	}
	return []stream{
		{1024, events, someOffsets(1024)},
		// 25 and 53 byte values make 28 and 56 byte records, which fill
		// their 32 byte frames; 0 is an empty payload; 200 takes a 2 byte
		// length.
		{32, bytesValues(5, 60, 25, 0, 53, 10, 200, 1), everyOffset(32)},
		// 9 and 21 byte values make 12 and 24 byte records, which fill
		// their 16 byte frames.
		{16, bytesValues(9, 1, 10, 11, 21, 30, 2), everyOffset(16)},
	}
}

func reads(w io.Writer) {
	for id, s := range streams() {
		var encoded [][]byte
		var items []string
		for _, m := range s.payloads {
			b, err := proto.Marshal(m)
			check(err)
			encoded = append(encoded, b)
			items = append(items, h(b))
		}
		b := writeStream(s.size, s.payloads)
		fmt.Fprintf(w, "stream %d %d %s %s\n", id, s.size, h(b), strings.Join(items, ","))

		for _, offset := range s.offsets(len(b)) {
			payloads, status := read(b, s.size, offset)
			items := []string{}
			for _, p := range payloads {
				item := "x" + hex.EncodeToString(p)
				for i, e := range encoded {
					if bytes.Equal(p, e) {
						item = strconv.Itoa(i)
						break
					}
				}
				items = append(items, item)
			}
			list := strings.Join(items, ",")
			if list == "" {
				list = "-"
			}
			fmt.Fprintf(w, "read %d %d %s %s\n", id, offset, status, list)
		}
	}
}

func main() {
	f, err := os.Create(filepath.Join("crates", "strims-chunkstream", "golden", "vectors.hex"))
	check(err)
	defer f.Close()
	w := bufio.NewWriter(f)
	defer w.Flush()

	frames(w)
	records(w)
	reads(w)
}
