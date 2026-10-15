// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

// Live pion peer for crates/strims-webrtc's interop tests.
//
// Run from the repo root:
//
//	go run -tags golden ./crates/strims-webrtc/golden raw  -role offer|answer [-size n] [-closer go|rust]
//	go run -tags golden ./crates/strims-webrtc/golden vnic -role offer|answer [-size n]
//
// Both modes bind pion's ICE UDP mux to 127.0.0.1 with vnic.NewWebRTCUDPMux
// (Go svc's UDPMuxAddress), so the only host candidate is on loopback, and
// use no STUN servers. Signalling is line-based on stdin/stdout:
//
//	stdout: ready                  (once the helper runs)
//	stdout: desc <json>            (the LinkDescription.description JSON)
//	stdin:  desc <json>
//	stdin:  got-all | bye          (raw mode, see below)
//
// "raw" configures pion exactly as vnic.NewWebRTCInterface and
// CreateLinkCandidate do (UDP4/UDP6, the UDP mux, an 8 MiB SCTP receive
// buffer, detached data channels, data channel "data" with id 1
// pre-negotiated, non-trickle via GatheringCompletePromise) and then uses
// the detached channel as vnic does (dcLink): it writes -size bytes of the
// Go stream in writes of at most 64 KiB and checks that it reads exactly
// -size bytes of the Rust stream. With -closer=go it then closes the data
// channel as dcLink.Close does, once Rust says "got-all"; with
// -closer=rust it waits for Rust to close the connection.
//
// "vnic" runs the real thing: vnic.NewWebRTCInterface and a vnic.Host, so
// the link carries PeerInit and frames. The peer script: echo port 1000
// on 1001, send -size bytes of the Go stream on port 3000 through a
// FrameWriter, finish on 9999. webRTCLinkCandidate never sets localDesc,
// so SetRemoteDescription returns false in both roles; the helper reports
// what it returned and waits for the peer handler.
//
// Reports go to stderr: "self", "set-remote <connected>", "peer",
// "recv-ok <n> <largest message>", "recv-bad <offset>", "eof", "done".
package main

import (
	"bufio"
	"bytes"
	"context"
	"crypto/ed25519"
	"encoding/binary"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"os"
	"strings"
	"sync"
	"time"

	"github.com/MemeLabs/strims/pkg/apis/type/key"
	vnicv1 "github.com/MemeLabs/strims/pkg/apis/vnic/v1"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/MemeLabs/strims/pkg/vnic/qos"
	"github.com/pion/webrtc/v3"
	"go.uber.org/zap"
)

// Stream seeds, shared with tests/cross_go.rs.
const (
	goStreamSeed   = 0x60
	rustStreamSeed = 0x52
	// dcMTU in webrtc_native.go.
	dcMTU = 64 * 1024
)

func main() {
	if len(os.Args) < 2 {
		fail(errors.New("usage: golden raw|vnic [flags]"))
	}
	var err error
	switch os.Args[1] {
	case "raw":
		err = runRaw(os.Args[2:])
	case "vnic":
		err = runVnic(os.Args[2:])
	default:
		err = fmt.Errorf("unknown mode %q", os.Args[1])
	}
	if err != nil {
		fail(err)
	}
}

func fail(err error) {
	fmt.Fprintln(os.Stderr, "error:", err)
	os.Exit(1)
}

func report(format string, args ...any) {
	fmt.Fprintf(os.Stderr, format+"\n", args...)
}

// stream is splitmix64 output, little-endian: the bytes each side sends.
type stream struct{ state uint64 }

func newStream(seed uint64) *stream { return &stream{state: seed} }

func (s *stream) next() uint64 {
	s.state += 0x9E3779B97F4A7C15
	z := s.state
	z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9
	z = (z ^ (z >> 27)) * 0x94D049BB133111EB
	return z ^ (z >> 31)
}

func streamBytes(seed uint64, n int) []byte {
	s := newStream(seed)
	b := make([]byte, (n+7)/8*8)
	for i := 0; i < len(b); i += 8 {
		binary.LittleEndian.PutUint64(b[i:], s.next())
	}
	return b[:n]
}

// --- signalling -------------------------------------------------------------

type signal struct {
	lines chan string
}

func newSignal() *signal {
	s := &signal{lines: make(chan string, 16)}
	go func() {
		sc := bufio.NewScanner(os.Stdin)
		sc.Buffer(make([]byte, 1<<20), 1<<20)
		for sc.Scan() {
			s.lines <- sc.Text()
		}
		close(s.lines)
	}()
	fmt.Println("ready")
	return s
}

func (s *signal) send(desc string) {
	fmt.Printf("desc %s\n", desc)
}

func (s *signal) expect(prefix string) (string, error) {
	select {
	case l, ok := <-s.lines:
		if !ok {
			return "", errors.New("stdin closed")
		}
		if !strings.HasPrefix(l, prefix) {
			return "", fmt.Errorf("expected %q, got %q", prefix, l)
		}
		return strings.TrimPrefix(strings.TrimPrefix(l, prefix), " "), nil
	case <-time.After(2 * time.Minute):
		return "", fmt.Errorf("timed out waiting for %q", prefix)
	}
}

// --- raw: pion configured as vnic.NewWebRTCInterface -------------------------

func ptr[T any](v T) *T { return &v }

func runRaw(args []string) error {
	fs := flag.NewFlagSet("raw", flag.ExitOnError)
	role := fs.String("role", "offer", "offer or answer")
	size := fs.Int("size", 4<<20, "bytes each way")
	closer := fs.String("closer", "rust", "who closes: go or rust")
	if err := fs.Parse(args); err != nil {
		return err
	}

	mux, lis, err := vnic.NewWebRTCUDPMux("127.0.0.1:0")
	if err != nil {
		return err
	}
	defer lis.Close()

	// As NewWebRTCInterface with a UDPMux.
	s := webrtc.SettingEngine{}
	s.SetICEUDPMux(mux)
	s.SetNetworkTypes([]webrtc.NetworkType{webrtc.NetworkTypeUDP4, webrtc.NetworkTypeUDP6})
	s.SetSCTPMaxReceiveBufferSize(8 * 1024 * 1024)
	s.DetachDataChannels()
	api := webrtc.NewAPI(webrtc.WithSettingEngine(s))

	// As CreateLinkCandidate, with no ICE servers.
	pc, err := api.NewPeerConnection(webrtc.Configuration{
		ICEServers: []webrtc.ICEServer{{URLs: []string{}}},
	})
	if err != nil {
		return err
	}
	defer pc.Close()
	pc.OnConnectionStateChange(func(s webrtc.PeerConnectionState) {
		report("state %s", s)
		if s == webrtc.PeerConnectionStateFailed {
			pc.Close()
		}
	})
	dc, err := pc.CreateDataChannel("data", &webrtc.DataChannelInit{
		ID:         ptr[uint16](1),
		Negotiated: ptr(true),
	})
	if err != nil {
		return err
	}
	opened := make(chan struct{})
	dc.OnOpen(func() { close(opened) })

	sig := newSignal()
	// As webRTCLinkCandidate.LocalDescription.
	local := func(answer bool) error {
		var desc webrtc.SessionDescription
		if answer {
			desc, err = pc.CreateAnswer(nil)
		} else {
			desc, err = pc.CreateOffer(nil)
		}
		if err != nil {
			return err
		}
		if err := pc.SetLocalDescription(desc); err != nil {
			return err
		}
		<-webrtc.GatheringCompletePromise(pc)
		b, err := json.Marshal(pc.LocalDescription())
		if err != nil {
			return err
		}
		sig.send(string(b))
		return nil
	}
	remote := func() error {
		line, err := sig.expect("desc")
		if err != nil {
			return err
		}
		var desc webrtc.SessionDescription
		if err := json.Unmarshal([]byte(line), &desc); err != nil {
			return err
		}
		return pc.SetRemoteDescription(desc)
	}
	if *role == "offer" {
		if err := local(false); err != nil {
			return err
		}
		if err := remote(); err != nil {
			return err
		}
	} else {
		if err := remote(); err != nil {
			return err
		}
		if err := local(true); err != nil {
			return err
		}
	}

	select {
	case <-opened:
	case <-time.After(30 * time.Second):
		return errors.New("data channel never opened")
	}
	rwc, err := dc.Detach()
	if err != nil {
		return err
	}
	report("open")

	go func() {
		data := streamBytes(goStreamSeed, *size)
		for off := 0; off < len(data); {
			n := len(data) - off
			if n > dcMTU {
				n = dcMTU
			}
			if _, err := rwc.Write(data[off : off+n]); err != nil {
				report("write error: %v", err)
				return
			}
			off += n
		}
		report("sent %d", *size)
	}()

	want := streamBytes(rustStreamSeed, *size)
	buf := make([]byte, 1<<20)
	got, largest := 0, 0
	for got < *size {
		n, err := rwc.Read(buf)
		if err != nil {
			return fmt.Errorf("read after %d bytes: %w", got, err)
		}
		if n > largest {
			largest = n
		}
		if got+n > *size || !bytes.Equal(buf[:n], want[got:got+n]) {
			report("recv-bad %d", got)
			return errors.New("received bytes differ")
		}
		got += n
	}
	report("recv-ok %d %d", got, largest)

	if *closer == "go" {
		if _, err := sig.expect("got-all"); err != nil {
			return err
		}
		// As dcLink.Close: only the data channel (an SCTP stream reset).
		if err := rwc.Close(); err != nil {
			return err
		}
		report("closed")
		if _, err := sig.expect("bye"); err != nil {
			return err
		}
	} else {
		// Rust closes the connection: the detached channel ends.
		n, err := rwc.Read(buf)
		if err == nil {
			return fmt.Errorf("read %d bytes after the stream", n)
		}
		report("eof %v", errors.Is(err, io.EOF))
		report("read error: %v", err)
	}
	report("done")
	return nil
}

// --- vnic: the real interface and host ---------------------------------------

func keyFromSeed(seed byte) *key.Key {
	priv := ed25519.NewKeyFromSeed(bytes.Repeat([]byte{seed}, ed25519.SeedSize))
	return &key.Key{
		Type:    key.KeyType_KEY_TYPE_ED25519,
		Private: priv,
		Public:  priv.Public().(ed25519.PublicKey),
	}
}

// kick enqueues a no-op packet every 2 ms, so qos.Control never waits on a
// lost wakeup.
func kick(c *qos.Class) {
	s := c.AddSession(1)
	go func() {
		for {
			time.Sleep(2 * time.Millisecond)
			s.Enqueue(nopPacket{})
		}
	}()
}

type nopPacket struct{}

func (nopPacket) Size() uint64 { return 1 }
func (nopPacket) Send()        {}

func runVnic(args []string) error {
	fs := flag.NewFlagSet("vnic", flag.ExitOnError)
	role := fs.String("role", "offer", "offer or answer")
	size := fs.Int("size", 2<<20, "bytes Go sends on port 3000")
	seed := fs.Int("seed", 0x42, "profile key seed byte")
	if err := fs.Parse(args); err != nil {
		return err
	}
	logger := zap.NewNop()

	mux, lis, err := vnic.NewWebRTCUDPMux("127.0.0.1:0")
	if err != nil {
		return err
	}
	defer lis.Close()
	iface := vnic.NewWebRTCInterface(logger, &vnic.WebRTCInterfaceOptions{
		ICEServers: []string{},
		UDPMux:     mux,
	})
	profile := keyFromSeed(byte(*seed))
	h, err := vnic.New(logger, profile, vnic.WithLabel("go-webrtc"), vnic.WithInterface(iface))
	if err != nil {
		return err
	}
	kick(h.QOS().Class)
	report("self %x", h.ID().Bytes(nil))

	done := make(chan struct{})
	var doneOnce sync.Once
	h.AddPeerHandler(vnic.PeerHandlerFunc(func(p *vnic.Peer) {
		report("peer %x", p.HostID().Bytes(nil))
		qc := h.QOS().AddClass(1)
		echo := vnic.NewFrameWriter(p.Link, 1001, qc)
		p.SetHandler(1000, func(_ *vnic.Peer, f vnic.Frame) error {
			_, err := echo.WriteFrame(append([]byte(nil), f.Body...))
			return err
		})
		p.SetHandler(9999, func(_ *vnic.Peer, f vnic.Frame) error {
			doneOnce.Do(func() { close(done) })
			return nil
		})
		go func() {
			fw := vnic.NewFrameWriter(p.Link, 3000, qc)
			if _, err := fw.Write(streamBytes(goStreamSeed, *size)); err != nil {
				report("write error: %v", err)
				return
			}
			if err := fw.Flush(); err != nil {
				report("flush error: %v", err)
				return
			}
			report("sent %d", *size)
		}()
	}))

	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Minute)
	defer cancel()
	c, err := iface.CreateLinkCandidate(ctx, h)
	if err != nil {
		return err
	}
	sig := newSignal()
	local := func() error {
		d, err := c.LocalDescription()
		if err != nil {
			return err
		}
		if d.Interface != "webrtc" {
			return fmt.Errorf("interface %q", d.Interface)
		}
		sig.send(d.Description)
		return nil
	}
	remote := func() error {
		line, err := sig.expect("desc")
		if err != nil {
			return err
		}
		connected, err := c.SetRemoteDescription(&vnicv1.LinkDescription{
			Interface:   "webrtc",
			Description: line,
		})
		if err != nil {
			return err
		}
		report("set-remote %v", connected)
		return nil
	}
	if *role == "offer" {
		if err := local(); err != nil {
			return err
		}
		if err := remote(); err != nil {
			return err
		}
	} else {
		if err := remote(); err != nil {
			return err
		}
		if err := local(); err != nil {
			return err
		}
	}

	select {
	case <-done:
	case <-ctx.Done():
		return errors.New("timed out")
	}
	report("done")
	return nil
}
