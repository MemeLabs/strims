// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

// A Go ppspp peer for the live Go<->Rust swarm tests in
// crates/strims-ppspp/tests/interop_go.rs.
//
// It runs a real ppspp.Runner with one peer, the Rust side, over
// stdin/stdout. Every frame body is prefixed with its length as a 4 byte
// big-endian integer. The peer's Conn is a plain buffer of one MTU that
// Flush writes out as a frame; vnic and its QoS are not involved.
//
//	go run -tags golden ./crates/strims-ppspp/golden/interop seed  -bytes N [-rate B] [-streams S]
//	go run -tags golden ./crates/strims-ppspp/golden/interop leech -bytes N [-streams S]
//
// Both sides derive the swarm key from a fixed seed and use the default
// swarm options (low latency delivery), with S streams if given (the push
// tests subscribe to them). The seed starts writing once the
// first frame from Rust arrives, N bytes at B bytes per second, generated
// by streamByte from the absolute stream offset, and exits when stdin
// closes. The leech reads from wherever its reader starts until offset N,
// checks every byte, and prints
//
//	result <start> <length> <sha256 hex> <mismatches>
//
// to stderr before exiting. Diagnostics go to stderr too.
package main

import (
	"bufio"
	"bytes"
	"context"
	"crypto/ed25519"
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"flag"
	"fmt"
	"io"
	"os"
	"sync"
	"time"

	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/ppspp/codec"
	"go.uber.org/zap"
)

const mtu = 16*1024 - 4

func streamByte(o uint64) byte {
	x := (o * 0x9e3779b97f4a7c15) ^ (o >> 7)
	return byte(x >> 56)
}

// conn is ppspp.Conn: a one-frame buffer that Flush writes to stdout.
type conn struct {
	mu  sync.Mutex
	out *bufio.Writer
	buf []byte
}

func newConn(w io.Writer) *conn {
	return &conn{out: bufio.NewWriterSize(w, 4*mtu), buf: make([]byte, 0, mtu)}
}

func (c *conn) Available() int          { return mtu - len(c.buf) }
func (c *conn) AvailableBuffer() []byte { return c.buf[len(c.buf):] }
func (c *conn) SetQOSWeight(w uint64)   {}
func (c *conn) Write(p []byte) (int, error) {
	c.buf = append(c.buf, p...)
	return len(p), nil
}

func (c *conn) Flush() error {
	if len(c.buf) == 0 {
		return nil
	}
	c.mu.Lock()
	defer c.mu.Unlock()
	var hdr [4]byte
	binary.BigEndian.PutUint32(hdr[:], uint32(len(c.buf)))
	c.out.Write(hdr[:])
	c.out.Write(c.buf)
	c.buf = c.buf[:0]
	return c.out.Flush()
}

func readFrames(r io.Reader, cr *ppspp.ChannelReader, first chan<- struct{}, done chan<- struct{}) {
	defer close(done)
	br := bufio.NewReaderSize(r, 4*mtu)
	var hdr [4]byte
	var once sync.Once
	for {
		if _, err := io.ReadFull(br, hdr[:]); err != nil {
			return
		}
		b := make([]byte, binary.BigEndian.Uint32(hdr[:]))
		if _, err := io.ReadFull(br, b); err != nil {
			return
		}
		once.Do(func() { close(first) })
		if err := cr.HandleMessage(b); err != nil {
			fmt.Fprintf(os.Stderr, "handle message: %v\n", err)
		}
	}
}

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: interop seed|leech -bytes N [-rate B]")
		os.Exit(2)
	}
	mode := os.Args[1]
	fs := flag.NewFlagSet(mode, flag.ExitOnError)
	total := fs.Uint64("bytes", 8<<20, "stream bytes")
	rate := fs.Uint64("rate", 2<<20, "seed bytes per second")
	verbose := fs.Bool("v", false, "log the runner")
	streams := fs.Int("streams", 0, "stream count (0: the default)")
	fs.Parse(os.Args[2:])

	go func() {
		time.Sleep(10 * time.Minute)
		fmt.Fprintln(os.Stderr, "timeout")
		os.Exit(3)
	}()

	priv := ed25519.NewKeyFromSeed(bytes.Repeat([]byte{9}, ed25519.SeedSize))
	k := &key.Key{Type: key.KeyType_KEY_TYPE_ED25519, Private: priv, Public: priv.Public().(ed25519.PublicKey)}

	logger := zap.NewNop()
	if *verbose {
		logger, _ = zap.NewDevelopment()
	}

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	runner := ppspp.NewRunner(ctx, logger)
	c := newConn(os.Stdout)
	cr, peer := runner.RunPeer([]byte("rust"), c)

	first := make(chan struct{})
	done := make(chan struct{})

	opts := ppspp.NewDefaultSwarmOptions()
	if *streams > 0 {
		opts.StreamCount = *streams
	}

	switch mode {
	case "seed":
		w, err := ppspp.NewWriter(ppspp.WriterOptions{SwarmOptions: opts, Key: k})
		if err != nil {
			panic(err)
		}
		if err := peer.RunSwarm(w.Swarm(), codec.Channel(1), codec.Channel(1)); err != nil {
			panic(err)
		}
		fmt.Fprintf(os.Stderr, "swarm %s\n", w.Swarm().ID())
		go readFrames(os.Stdin, cr, first, done)

		<-first
		const unit = 64 * 1024
		ivl := time.Duration(float64(time.Second) * unit / float64(*rate))
		b := make([]byte, unit)
		t := time.NewTicker(ivl)
		for off := uint64(0); off < *total; off += unit {
			<-t.C
			for i := range b {
				b[i] = streamByte(off + uint64(i))
			}
			if _, err := w.Write(b); err != nil {
				panic(err)
			}
		}
		t.Stop()
		fmt.Fprintln(os.Stderr, "written")
		<-done

	case "leech":
		swarm, err := ppspp.NewSwarm(ppspp.NewSwarmID(k.Public), opts)
		if err != nil {
			panic(err)
		}
		if err := peer.RunSwarm(swarm, codec.Channel(1), codec.Channel(1)); err != nil {
			panic(err)
		}
		fmt.Fprintf(os.Stderr, "swarm %s\n", swarm.ID())
		go readFrames(os.Stdin, cr, first, done)

		r := swarm.Reader()
		start := r.Offset()
		fmt.Fprintf(os.Stderr, "offset %d\n", start)
		h := sha256.New()
		var mismatches uint64
		off := start
		b := make([]byte, 64*1024)
		for off < *total {
			n, err := r.Read(b)
			if err != nil {
				fmt.Fprintf(os.Stderr, "read at %d: %v\n", off, err)
				os.Exit(1)
			}
			if rest := *total - off; uint64(n) > rest {
				n = int(rest)
			}
			for i := 0; i < n; i++ {
				if b[i] != streamByte(off+uint64(i)) {
					mismatches++
				}
			}
			h.Write(b[:n])
			off += uint64(n)
		}
		fmt.Fprintf(os.Stderr, "result %d %d %s %d\n", start, off-start, hex.EncodeToString(h.Sum(nil)), mismatches)

	default:
		fmt.Fprintln(os.Stderr, "unknown mode", mode)
		os.Exit(2)
	}
}
