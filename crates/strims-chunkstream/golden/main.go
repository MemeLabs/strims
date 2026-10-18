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
//	packed <id> <size> <op>,<op>,... <stream>
//	    a script through one chunkstream.Writer of that size. An op is
//	    "w<n>:<out>" (Write the script's next n bytes), "e:<out>" (Flush) or
//	    "E:<out>" (a Flush that failed with ErrHeaderWritten), where out is
//	    the stream length the Writer has written after the op. The
//	    script's bytes are pattern(total bytes written).
//	packed_digest <id> <size> <op>,<op>,... <stream sha256> <headers>
//	    the same for large frames: headers concatenates each frame's first
//	    4 bytes
//	packed_read <id> <offset> <len>,<len>,...|- <tail> <digest>
//	    what chunkstream.Reader returns when its input starts at byte offset
//	    of packed stream id: the length of each record it ends with io.EOF,
//	    the bytes it returns after the last of them before the input runs
//	    out, and the first 8 bytes of the sha256 of all the bytes it
//	    returns. Offsets inside a frame header are left out, as for read.
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

// flush is a script op that ends the record.
const flush = -1

// packedScripts are the scripts written with frames of size bytes: write
// lengths, and flush.
func packedScripts(size int) [][]int {
	d := size - 4
	return [][]int{
		// An empty record, then ends refused after an end in the frame.
		{flush, 1, flush, d - 1, flush, 2, flush},
		// Records that fill their frames, so their ends land at offset 4.
		{d, flush, d, flush, 3, flush, d - 3, flush, 2*d + 7},
		// Records sharing frames, some written in several pieces.
		{3, flush, 4, flush, d, flush, 1, 0, 2, flush, d - 1, 5, flush, 2},
		// Records over many frames.
		{3*d + 5, flush, 2 * d, flush, 1, flush, d, flush},
		randomScript(size, 40),
	}
}

// randomScript is a seeded mix of flushes, writes that fill the frame and
// writes of up to two frames.
func randomScript(size, ops int) (script []int) {
	d := size - 4
	state := uint64(size)
	next := func(n int) int {
		state = state*6364136223846793005 + 1442695040888963407
		return int(state>>33) % n
	}
	written := 0
	for i := 0; i < ops; i++ {
		n := flush
		switch next(6) {
		case 0, 1:
		case 2:
			n = d - written%d
		case 3:
			n = next(5)
		default:
			n = next(2 * size)
		}
		if n >= 0 {
			written += n
		}
		script = append(script, n)
	}
	return script
}

// runScript writes script through a chunkstream.Writer and returns the
// stream and each op with the stream length after it.
func runScript(size int, script []int) (stream []byte, ops []string) {
	var b bytes.Buffer
	w, err := chunkstream.NewWriterSize(&b, size)
	check(err)
	written := 0
	for _, n := range script {
		switch {
		case n == flush:
			err := w.Flush()
			switch {
			case err == nil:
				ops = append(ops, fmt.Sprintf("e:%d", b.Len()))
			case errors.Is(err, chunkstream.ErrHeaderWritten):
				ops = append(ops, fmt.Sprintf("E:%d", b.Len()))
			default:
				panic(err)
			}
		default:
			_, err := w.Write(pattern(written + n)[written:])
			check(err)
			written += n
			ops = append(ops, fmt.Sprintf("w%d:%d", n, b.Len()))
		}
	}
	return b.Bytes(), ops
}

func packedRead(stream []byte, size, offset int) (records []string, tail int, digest []byte) {
	r, err := chunkstream.NewReaderSize(&input{b: stream[offset:], offset: uint64(offset)}, int64(offset), size)
	check(err)
	sum := sha256.New()
	p := make([]byte, 64)
	for {
		n, err := r.Read(p)
		if errors.Is(err, errEnd) {
			return records, tail, sum.Sum(nil)[:8]
		}
		sum.Write(p[:n])
		tail += n
		switch {
		case err == io.EOF:
			records = append(records, strconv.Itoa(tail))
			tail = 0
		case err != nil:
			panic(err)
		}
	}
}

func packed(w io.Writer) {
	id := 0
	for _, size := range []int{16, 32, 1024} {
		offsets := everyOffset(size)
		if size == 1024 {
			offsets = someOffsets(size)
		}
		for _, script := range packedScripts(size) {
			b, ops := runScript(size, script)
			fmt.Fprintf(w, "packed %d %d %s %s\n", id, size, strings.Join(ops, ","), h(b))
			for _, offset := range offsets(len(b)) {
				records, tail, digest := packedRead(b, size, offset)
				list := strings.Join(records, ",")
				if list == "" {
					list = "-"
				}
				fmt.Fprintf(w, "packed_read %d %d %s %d %s\n", id, offset, list, tail, h(digest))
			}
			id++
		}
	}
	size := chunkstream.DefaultSize
	for _, script := range packedScripts(size) {
		b, ops := runScript(size, script)
		var headers []byte
		for i := 0; i < len(b); i += size {
			headers = append(headers, b[i:i+4]...)
		}
		sum := sha256.Sum256(b)
		fmt.Fprintf(w, "packed_digest %d %d %s %s %s\n", id, size, strings.Join(ops, ","), h(sum[:]), h(headers))
		id++
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
	packed(w)
}
