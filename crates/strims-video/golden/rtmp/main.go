// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

// A joy5 RTMP publisher for crates/strims-video/tests/cross_go.rs.
//
//	go run -tags golden ./crates/strims-video/golden/rtmp <addr> <name> <script>
//
// It connects to <addr> with joy5's client, publishes
// rtmp://<addr>/live/<name>, sends the script's tags with Conn.WriteTag,
// then FCUnpublish and deleteStream, and closes. Each script line is one
// tag:
//
//	video <timestamp> <hex FLV tag body>
//	audio <timestamp> <hex FLV tag body>
//	metadata <key>=<value> ...
//
// A metadata value that parses as a float is a number, else a string; the
// tag is "@setDataFrame" "onMetaData" and an object of the pairs in order.
//
// It prints each tag as sent, one per line, as "video <timestamp> <hex
// body>", "audio <timestamp> <hex body>" or "metadata <hex message>", then
// "s1 <true|false>": whether joy5's hsParse1 accepted the server's S1
// digest, and "s2 <true|false>": whether S2 ends with the HMAC over its
// first 1504 bytes keyed by the HMAC of C1's digest under the server key.
package main

import (
	"bufio"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"net"
	"net/url"
	"os"
	"strconv"
	"strings"

	"github.com/nareix/joy5/format/flv/flvio"
	"github.com/nareix/joy5/format/rtmp"

	_ "unsafe" // go:linkname
)

// joy5's handshake internals, linked so the checks are its own.

//go:linkname hsParse1 github.com/nareix/joy5/format/rtmp.hsParse1
func hsParse1(p []byte, peerkey []byte, key []byte) (ok bool, digest []byte)

//go:linkname hsFindDigest github.com/nareix/joy5/format/rtmp.hsFindDigest
func hsFindDigest(p []byte, key []byte, base int) int

//go:linkname hsClientFullKey github.com/nareix/joy5/format/rtmp.hsClientFullKey
var hsClientFullKey []byte

//go:linkname hsServerFullKey github.com/nareix/joy5/format/rtmp.hsServerFullKey
var hsServerFullKey []byte

//go:linkname hsClientPartialKey github.com/nareix/joy5/format/rtmp.hsClientPartialKey
var hsClientPartialKey []byte

//go:linkname hsServerPartialKey github.com/nareix/joy5/format/rtmp.hsServerPartialKey
var hsServerPartialKey []byte

const packetLen = 1536

// recorder keeps the first bytes each way: C0 C1 out, S0 S1 S2 in.
type recorder struct {
	net.Conn
	sent, received []byte
}

// keep appends to *dst up to limit bytes; once full, *dst is only read.
func keep(dst *[]byte, b []byte, limit int) {
	room := limit - len(*dst)
	if room <= 0 {
		return
	}
	if len(b) < room {
		room = len(b)
	}
	*dst = append(*dst, b[:room]...)
}

func (r *recorder) Read(b []byte) (int, error) {
	n, err := r.Conn.Read(b)
	keep(&r.received, b[:n], 1+2*packetLen)
	return n, err
}

func (r *recorder) Write(b []byte) (int, error) {
	keep(&r.sent, b, 1+packetLen)
	return r.Conn.Write(b)
}

type readWriteFlusher struct {
	*bufio.Reader
	*bufio.Writer
}

func fail(format string, args ...interface{}) {
	fmt.Fprintf(os.Stderr, format+"\n", args...)
	os.Exit(1)
}

func readScript(path string) []flvio.Tag {
	f, err := os.ReadFile(path)
	if err != nil {
		fail("read script: %v", err)
	}
	var tags []flvio.Tag
	for _, line := range strings.Split(strings.TrimSpace(string(f)), "\n") {
		fields := strings.Fields(line)
		switch fields[0] {
		case "video", "audio":
			ts, err := strconv.ParseUint(fields[1], 10, 32)
			if err != nil {
				fail("timestamp %q: %v", fields[1], err)
			}
			body, err := hex.DecodeString(fields[2])
			if err != nil {
				fail("body: %v", err)
			}
			tag := flvio.Tag{Type: flvio.TAG_VIDEO, Time: uint32(ts)}
			if fields[0] == "audio" {
				tag.Type = flvio.TAG_AUDIO
			}
			if err := tag.Parse(body); err != nil {
				fail("parse %s tag: %v", fields[0], err)
			}
			tags = append(tags, tag)
		case "metadata":
			var m flvio.AMFMap
			for _, kv := range fields[1:] {
				k, v, _ := strings.Cut(kv, "=")
				if n, err := strconv.ParseFloat(v, 64); err == nil {
					m = append(m, flvio.AMFKv{K: k, V: n})
				} else {
					m = append(m, flvio.AMFKv{K: k, V: v})
				}
			}
			data := flvio.FillAMF0ValsMalloc([]interface{}{"@setDataFrame", "onMetaData", m})
			tags = append(tags, flvio.Tag{Type: flvio.TAG_AMF0, Data: data})
		default:
			fail("unknown script line %q", line)
		}
	}
	return tags
}

func describe(t flvio.Tag) string {
	switch t.Type {
	case flvio.TAG_VIDEO, flvio.TAG_AUDIO:
		h := make([]byte, t.MaxHeaderLen())
		body := append(h[:t.FillHeader(h)], t.Data...)
		kind := "video"
		if t.Type == flvio.TAG_AUDIO {
			kind = "audio"
		}
		return fmt.Sprintf("%s %d %x", kind, t.Time, body)
	default:
		return fmt.Sprintf("metadata %x", t.Data)
	}
}

func main() {
	if len(os.Args) != 4 {
		fail("usage: rtmp <addr> <name> <script>")
	}
	addr, name, script := os.Args[1], os.Args[2], os.Args[3]
	tags := readScript(script)

	nc, err := net.Dial("tcp", addr)
	if err != nil {
		fail("dial: %v", err)
	}
	rec := &recorder{Conn: nc}
	rw := &readWriteFlusher{bufio.NewReader(rec), bufio.NewWriter(rec)}
	c := rtmp.NewConn(rw)
	if c.URL, err = url.Parse("rtmp://" + addr + "/live/" + name); err != nil {
		fail("url: %v", err)
	}
	if err := c.Prepare(rtmp.StageGotPublishOrPlayCommand, rtmp.PrepareWriting); err != nil {
		fail("publish: %v", err)
	}
	for _, t := range tags {
		if err := c.WriteTag(t); err != nil {
			fail("write: %v", err)
		}
		fmt.Println(describe(t))
	}
	for _, cmd := range [][]interface{}{
		{"FCUnpublish", 6.0, nil, name},
		{"deleteStream", 7.0, nil, 1.0},
	} {
		if err := c.WriteTag(flvio.Tag{Type: 20, Data: flvio.FillAMF0ValsMalloc(cmd)}); err != nil {
			fail("write: %v", err)
		}
	}
	if err := rw.Flush(); err != nil {
		fail("flush: %v", err)
	}

	if len(rec.sent) != 1+packetLen || len(rec.received) != 1+2*packetLen {
		fail("handshake not recorded")
	}
	c1 := rec.sent[1:]
	s1 := rec.received[1 : 1+packetLen]
	s2 := rec.received[1+packetLen:]
	s1ok, _ := hsParse1(s1, hsServerPartialKey, hsClientFullKey)
	fmt.Println("s1", s1ok)
	s2ok := false
	if pos := hsFindDigest(c1, hsClientPartialKey, 8); pos >= 0 {
		key := hmac.New(sha256.New, hsServerFullKey)
		key.Write(c1[pos : pos+32])
		mac := hmac.New(sha256.New, key.Sum(nil))
		mac.Write(s2[:packetLen-32])
		s2ok = hmac.Equal(mac.Sum(nil), s2[packetLen-32:])
	}
	fmt.Println("s2", s2ok)
	nc.Close()
}
