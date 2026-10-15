// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

// Golden-vector generator and live vnic peer for crates/strims-vnic.
//
// Run from the repo root:
//
//	go run -tags golden ./crates/strims-vnic/golden vectors > crates/strims-vnic/golden/vectors.txt
//	go run -tags golden ./crates/strims-vnic/golden live [-seed n] [-mtu n] [-aes none|dial|accept] [-peer hex]
//	go run -tags golden ./crates/strims-vnic/golden tcp-listen [-seed n] [-read-timeout d]
//	go run -tags golden ./crates/strims-vnic/golden tcp-dial [-seed n] [-read-timeout d] -uri tcp://...
//
// "live" runs a real vnic.Host over a link on stdin/stdout. With -aes=dial
// it first writes a TCPMuxInit for -peer (an ed25519 profile key) and runs
// the AES link handshake expecting that key, as tcpInterface.Dial does; with
// -aes=accept it reads the TCPMuxInit and runs the handshake as the
// listener. "tcp-listen" and "tcp-dial" use the real TCP interface and mux
// on 127.0.0.1, with -read-timeout as TCPInterfaceOptions.ReadTimeout (the
// per-read deadline; 0 keeps Go's default of 25s). In every mode the peer
// runs the script in setupPeer and reports to stderr:
//
//	self <host id> <profile key>
//	uri <uri>                       (tcp-listen)
//	peer <peer host id> <peer profile key>
//	recv 2000 <body>
//	done
//	closed                          (the peer closed before done; exits 1)
//
// Vector lines are "<kind> key=value ..." with hex values.
package main

import (
	"bytes"
	"context"
	"crypto/ed25519"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"net"
	"os"
	"strings"
	"sync"
	"time"
	_ "unsafe" // go:linkname

	"github.com/MemeLabs/strims/pkg/apis/type/key"
	vnicv1 "github.com/MemeLabs/strims/pkg/apis/vnic/v1"
	"github.com/MemeLabs/strims/pkg/ed25519util"
	"github.com/MemeLabs/strims/pkg/httputil"
	"github.com/MemeLabs/strims/pkg/protoutil"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/MemeLabs/strims/pkg/vnic/qos"
	"github.com/pion/webrtc/v3"
	"go.uber.org/zap"
)

//go:linkname handshakeAESLink github.com/MemeLabs/strims/pkg/vnic.handshakeAESLink
func handshakeAESLink(l vnic.Link, k *key.Key, peerKey []byte) (vnic.Link, error)

func main() {
	if len(os.Args) < 2 {
		fail(errors.New("usage: golden vectors|live|tcp-listen|tcp-dial [flags]"))
	}
	var err error
	switch os.Args[1] {
	case "vectors":
		err = writeVectors(os.Stdout)
	case "live", "tcp-listen", "tcp-dial":
		err = runPeer(os.Args[1], os.Args[2:])
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

func keyFromSeed(seed byte) *key.Key {
	priv := ed25519.NewKeyFromSeed(bytes.Repeat([]byte{seed}, ed25519.SeedSize))
	return &key.Key{
		Type:    key.KeyType_KEY_TYPE_ED25519,
		Private: priv,
		Public:  priv.Public().(ed25519.PublicKey),
	}
}

func pattern(n int, salt byte) []byte {
	b := make([]byte, n)
	for i := range b {
		b[i] = byte(i*7) ^ salt
	}
	return b
}

// kick enqueues a no-op packet periodically. qos.Control can leave an
// enqueued packet unreleased until a later Enqueue (Session.Enqueue wakes the
// run loop with a non-blocking send the loop may miss), so this bounds the
// stall.
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

func hexList(bs [][]byte) string {
	s := make([]string, len(bs))
	for i, b := range bs {
		s[i] = hex.EncodeToString(b)
	}
	return strings.Join(s, ",")
}

// --- vectors ---------------------------------------------------------------

func writeVectors(w io.Writer) error {
	writeFrames(w)
	if err := writeFrameWriters(w); err != nil {
		return err
	}
	if err := writeTCPMuxInits(w); err != nil {
		return err
	}
	if err := writeAESLinks(w); err != nil {
		return err
	}
	if err := writePeerInits(w); err != nil {
		return err
	}
	if err := writeURIs(w); err != nil {
		return err
	}
	return writeSDPs(w)
}

func writeFrames(w io.Writer) {
	cases := []struct {
		port uint16
		body []byte
	}{
		{0, nil},
		{1, []byte{0}},
		{0xffff, []byte{0xab, 0xcd, 0xef}},
		{1234, pattern(300, 1)},
		{0x0102, pattern(4000, 2)},
	}
	for _, c := range cases {
		f := vnic.Frame{
			Header: vnic.FrameHeader{Port: c.port, Length: uint16(len(c.body))},
			Body:   c.body,
		}
		b := make([]byte, 4+len(c.body))
		f.Marshal(b)
		fmt.Fprintf(w, "frame port=%d body=%x wire=%x\n", c.port, c.body, b)
	}
}

// recLink records each Write as one entry.
type recLink struct {
	mu     sync.Mutex
	mtu    int
	writes [][]byte
}

func (l *recLink) Read(p []byte) (int, error) { return 0, io.EOF }
func (l *recLink) Close() error               { return nil }
func (l *recLink) MTU() int                   { return l.mtu }
func (l *recLink) Write(p []byte) (int, error) {
	l.mu.Lock()
	defer l.mu.Unlock()
	l.writes = append(l.writes, append([]byte(nil), p...))
	return len(p), nil
}

func writeFrameWriters(w io.Writer) error {
	type op struct {
		kind string // "w", "f", "wf"
		data []byte
	}
	cases := []struct {
		mtu  int
		port uint16
		ops  []op
	}{
		{12, 7, []op{{"w", []byte("ab")}, {"f", nil}, {"f", nil}, {"w", []byte("cdefghijklmnopqrstu")}, {"w", []byte("v")}, {"wf", []byte("wxyz")}}},
		{8, 0xfffe, []op{{"wf", pattern(9, 3)}, {"wf", nil}, {"w", pattern(4, 4)}, {"f", nil}}},
		{1200, 3000, []op{{"w", pattern(3000, 5)}, {"f", nil}, {"wf", []byte("go")}}},
	}
	qc := qos.New().AddClass(1)
	kick(qc)
	for _, c := range cases {
		l := &recLink{mtu: c.mtu}
		fw := vnic.NewFrameWriter(l, c.port, qc)
		var ops []string
		for _, o := range c.ops {
			var err error
			switch o.kind {
			case "w":
				_, err = fw.Write(o.data)
			case "f":
				err = fw.Flush()
			case "wf":
				_, err = fw.WriteFrame(o.data)
			}
			if err != nil {
				return err
			}
			if o.kind == "f" {
				ops = append(ops, "f")
			} else {
				ops = append(ops, o.kind+":"+hex.EncodeToString(o.data))
			}
		}
		fw.Close()
		fmt.Fprintf(w, "framewriter mtu=%d port=%d ops=%s writes=%s\n", c.mtu, c.port, strings.Join(ops, ","), hexList(l.writes))
	}
	return nil
}

func writeTCPMuxInits(w io.Writer) error {
	ln, err := net.ListenTCP("tcp", &net.TCPAddr{IP: net.IPv4(127, 0, 0, 1)})
	if err != nil {
		return err
	}
	defer ln.Close()
	for _, seed := range []byte{1, 2} {
		peerKey := keyFromSeed(seed).Public
		got := make(chan []byte, 1)
		go func() {
			c, err := ln.Accept()
			if err != nil {
				got <- nil
				return
			}
			b, _ := io.ReadAll(c)
			got <- b
		}()
		c, err := vnic.DialTCPMux(ln.Addr().(*net.TCPAddr), peerKey)
		if err != nil {
			return err
		}
		c.Close()
		fmt.Fprintf(w, "tcpmuxinit peer_key=%x wire=%x\n", peerKey, <-got)
	}
	return nil
}

// pipeEnd is one side of a buffered in-memory link that records its writes.
type pipeEnd struct {
	mtu   int
	in    *pipeBuf
	out   *pipeBuf
	mu    sync.Mutex
	wrote []byte
}

type pipeBuf struct {
	mu   sync.Mutex
	cond *sync.Cond
	buf  bytes.Buffer
}

func newPipeBuf() *pipeBuf {
	b := &pipeBuf{}
	b.cond = sync.NewCond(&b.mu)
	return b
}

func newPipe(mtu int) (*pipeEnd, *pipeEnd) {
	ab, ba := newPipeBuf(), newPipeBuf()
	return &pipeEnd{mtu: mtu, in: ba, out: ab}, &pipeEnd{mtu: mtu, in: ab, out: ba}
}

func (p *pipeEnd) Read(b []byte) (int, error) {
	p.in.mu.Lock()
	defer p.in.mu.Unlock()
	for p.in.buf.Len() == 0 {
		p.in.cond.Wait()
	}
	return p.in.buf.Read(b)
}

func (p *pipeEnd) Write(b []byte) (int, error) {
	p.mu.Lock()
	p.wrote = append(p.wrote, b...)
	p.mu.Unlock()
	p.out.mu.Lock()
	defer p.out.mu.Unlock()
	p.out.buf.Write(b)
	p.out.cond.Broadcast()
	return len(b), nil
}

func (p *pipeEnd) Close() error { return nil }
func (p *pipeEnd) MTU() int     { return p.mtu }

func (p *pipeEnd) written() []byte {
	p.mu.Lock()
	defer p.mu.Unlock()
	return append([]byte(nil), p.wrote...)
}

func (p *pipeEnd) waitWritten(n int) {
	for len(p.written()) < n {
		time.Sleep(time.Millisecond)
	}
}

// ivReader hands out the queued IVs to crypto/rand.Read, in order.
type ivReader struct {
	mu  sync.Mutex
	ivs [][]byte
}

func (r *ivReader) Read(b []byte) (int, error) {
	r.mu.Lock()
	defer r.mu.Unlock()
	if len(r.ivs) == 0 {
		return 0, errors.New("no iv queued")
	}
	n := copy(b, r.ivs[0])
	r.ivs = r.ivs[1:]
	return n, nil
}

func writeAESLinks(w io.Writer) error {
	saved := rand.Reader
	defer func() { rand.Reader = saved }()

	cases := []struct {
		aSeed, bSeed byte
		aPlain       [][]byte
		bPlain       [][]byte
	}{
		{0x11, 0x22, [][]byte{[]byte("hello"), pattern(16, 1), pattern(33, 2), {}, pattern(1, 3)}, [][]byte{pattern(100, 4)}},
		{0x33, 0x44, [][]byte{pattern(5000, 5)}, [][]byte{[]byte("x"), []byte("yz"), pattern(15, 6), pattern(17, 7)}},
	}
	for i, c := range cases {
		aIV, bIV := pattern(16, byte(0x80+i)), pattern(16, byte(0x90+i))
		rand.Reader = &ivReader{ivs: [][]byte{aIV, bIV}}
		aKey, bKey := keyFromSeed(c.aSeed), keyFromSeed(c.bSeed)
		var bx [32]byte
		ed25519util.PublicKeyToCurve25519(&bx, (*[32]byte)(bKey.Public))

		pa, pb := newPipe(65535)
		var la, lb vnic.Link
		var errA, errB error
		var wg sync.WaitGroup
		wg.Add(2)
		go func() {
			defer wg.Done()
			la, errA = handshakeAESLink(pa, ed25519util.KeyToCurve25519(aKey), bx[:])
		}()
		// B draws its IV only after A drew its own.
		pa.waitWritten(1)
		go func() {
			defer wg.Done()
			lb, errB = handshakeAESLink(pb, ed25519util.KeyToCurve25519(bKey), nil)
		}()
		wg.Wait()
		if errA != nil || errB != nil {
			return fmt.Errorf("aes handshake: %v %v", errA, errB)
		}
		if err := exchange(la, lb, c.aPlain); err != nil {
			return err
		}
		if err := exchange(lb, la, c.bPlain); err != nil {
			return err
		}
		fmt.Fprintf(w, "aes a_seed=%x b_seed=%x a_iv=%x b_iv=%x a_plain=%s b_plain=%s a_wire=%x b_wire=%x\n",
			bytes.Repeat([]byte{c.aSeed}, 32), bytes.Repeat([]byte{c.bSeed}, 32), aIV, bIV,
			hexList(c.aPlain), hexList(c.bPlain), pa.written(), pb.written())
	}
	return nil
}

// exchange writes each chunk to from and checks to reads them back.
func exchange(from, to vnic.Link, chunks [][]byte) error {
	want := bytes.Join(chunks, nil)
	for _, c := range chunks {
		if _, err := from.Write(c); err != nil {
			return err
		}
	}
	got := make([]byte, len(want))
	if _, err := io.ReadFull(to, got); err != nil {
		return err
	}
	if !bytes.Equal(got, want) {
		return errors.New("aes link round trip mismatch")
	}
	return nil
}

func writePeerInits(w io.Writer) error {
	for _, c := range []struct {
		seed  byte
		label string
	}{{0x55, "golden-a"}, {0x66, ""}} {
		ka := keyFromSeed(c.seed)
		ha, err := vnic.New(zap.NewNop(), ka, vnic.WithLabel(c.label))
		if err != nil {
			return err
		}
		hb, err := vnic.New(zap.NewNop(), keyFromSeed(c.seed+1))
		if err != nil {
			return err
		}
		pa, pb := newPipe(1500)
		now := time.Now()
		var wg sync.WaitGroup
		wg.Add(1)
		var peerB *vnic.Peer
		go func() {
			defer wg.Done()
			peerB, _ = hb.AddLink(pb)
		}()
		peerA, _ := ha.AddLink(pa)
		wg.Wait()
		if peerA == nil || peerB == nil || peerB.HostID() != ha.ID() {
			return errors.New("go peer init failed")
		}
		fmt.Fprintf(w, "peerinit now=%d host_id=%x profile_key=%x label=%x wire=%x\n",
			now.UnixNano(), ha.ID().Bytes(nil), ka.Public, c.label, pa.written())
		ha.Close()
		hb.Close()
	}
	return nil
}

func writeURIs(w io.Writer) error {
	profile := keyFromSeed(0x77)
	h, err := vnic.New(zap.NewNop(), profile)
	if err != nil {
		return err
	}
	desc := func(iface vnic.Interface) (string, error) {
		if err := iface.(vnic.Listener).Listen(h); err != nil {
			return "", err
		}
		c, err := iface.CreateLinkCandidate(context.Background(), h)
		if err != nil {
			return "", err
		}
		d, err := c.LocalDescription()
		if err != nil || d == nil {
			return "", err
		}
		return d.Description, nil
	}
	tcpCases := []struct{ address, hostIP string }{
		{"1.2.3.4:5", ""},
		{"127.0.0.1:65535", ""},
		{"0.0.0.0:8080", ""},
		{"0.0.0.0:8080", "10.0.0.1"},
		{"[::1]:9", ""},
		{"[::]:9", ""},
		{"[::]:9", "2001:db8::1"},
		{"[::ffff:1.2.3.4]:80", ""},
	}
	for _, c := range tcpCases {
		out, err := desc(vnic.NewTCPInterface(zap.NewNop(), vnic.TCPInterfaceOptions{
			Address: c.address,
			HostIP:  c.hostIP,
			Mux:     &vnic.TCPMux{},
		}))
		if err != nil {
			return err
		}
		fmt.Fprintf(w, "tcpuri address=%x host_ip=%x key=%x out=%x\n", c.address, c.hostIP, profile.Public, out)
	}
	wsCases := []struct {
		address, hostname string
		port              uint16
		secure            bool
	}{
		{"1.2.3.4:80", "", 0, false},
		{"example.com:80", "", 443, true},
		{"0.0.0.0:80", "strims.gg", 0, true},
		{"[::1]:80", "", 0, false},
		{"0.0.0.0:80", "", 0, false},
		{":80", "", 0, false},
		{"[::]:8443", "", 0, true},
		{"[::]:8443", "2001:db8::2", 8080, false},
	}
	for _, c := range wsCases {
		out, err := desc(vnic.NewWSInterface(zap.NewNop(), vnic.WSInterfaceOptions{
			ServeMux:       httputil.NewMapServeMux(),
			Address:        c.address,
			PublicHostname: c.hostname,
			PublicPort:     c.port,
			Secure:         c.secure,
		}))
		if err != nil {
			return err
		}
		fmt.Fprintf(w, "wsuri address=%x public_hostname=%x public_port=%d secure=%t key=%x out=%x\n",
			c.address, c.hostname, c.port, c.secure, profile.Public, out)
	}
	return nil
}

func writeSDPs(w io.Writer) error {
	special := "v=0\r\no=- 46117 2 IN IP4 127.0.0.1\r\na=<&>" + string(rune(0x2028)) + string(rune(0x2029)) +
		"\"\\/\x01\x08\x0c\x1f\t" + string(rune(0xe9)) + string(rune(0x1f600)) + "\r\n"
	descs := []webrtc.SessionDescription{
		{Type: webrtc.SDPTypeOffer, SDP: special},
		{Type: webrtc.SDPTypeAnswer, SDP: ""},
		{Type: webrtc.SDPTypePranswer, SDP: "a=x\r\n"},
		{Type: webrtc.SDPTypeRollback, SDP: "z"},
	}
	for _, d := range descs {
		b, err := json.Marshal(d)
		if err != nil {
			return err
		}
		fmt.Fprintf(w, "sdp type=%s sdp=%x json=%x\n", d.Type, d.SDP, b)
	}
	bs := "\\"
	inputs := []string{
		`{"type":"answer","sdp":"x"}`,
		` { "SDP" : "a` + bs + `/b` + bs + `ud83d` + bs + `ude00` + bs + `ud800x" , "extra":[1,-2.5e3,{"k":null},true], "Type":"OFFER" } `,
		`{"type":"offer","type":"pranswer","sdp":null}`,
		`{"sdp":"x"}`,
		`{"type":"bogus","sdp":"x"}`,
		`{"type":null,"sdp":"x"}`,
		`{"type":"offer","sdp":1}`,
		`{"type":"offer"} x`,
		`{"type":"rollback","sdp":"` + bs + `u003c` + bs + `u0026` + bs + `b` + bs + `f"}`,
		`[]`,
	}
	for _, in := range inputs {
		var d webrtc.SessionDescription
		err := json.Unmarshal([]byte(in), &d)
		fmt.Fprintf(w, "sdpdecode json=%x ok=%t type=%s sdp=%x\n", in, err == nil, d.Type, d.SDP)
	}
	return nil
}

// --- live peers ------------------------------------------------------------

type stdioLink struct{ mtu int }

func (l *stdioLink) Read(p []byte) (int, error)  { return os.Stdin.Read(p) }
func (l *stdioLink) Write(p []byte) (int, error) { return os.Stdout.Write(p) }
func (l *stdioLink) Close() error                { return nil }
func (l *stdioLink) MTU() int                    { return l.mtu }

func runPeer(mode string, args []string) error {
	fs := flag.NewFlagSet(mode, flag.ExitOnError)
	seed := fs.Int("seed", 0x42, "profile key seed byte")
	mtu := fs.Int("mtu", 1200, "stdio link mtu")
	aes := fs.String("aes", "none", "live: none, dial or accept")
	peerHex := fs.String("peer", "", "live -aes=dial: the listener's ed25519 profile key")
	uri := fs.String("uri", "", "tcp-dial: the uri to dial")
	label := fs.String("label", "go-peer", "host label")
	readTimeout := fs.Duration("read-timeout", 0, "tcp modes: per-read deadline (0: Go's default)")
	if err := fs.Parse(args); err != nil {
		return err
	}
	logger := zap.NewNop()
	profile := keyFromSeed(byte(*seed))

	var opts []vnic.HostOption
	opts = append(opts, vnic.WithLabel(*label))
	var tcpIface vnic.Interface
	var ln *net.TCPListener
	switch mode {
	case "tcp-listen":
		mux, l, err := vnic.NewTCPMux(logger, "127.0.0.1:0")
		if err != nil {
			return err
		}
		ln = l
		tcpIface = vnic.NewTCPInterface(logger, vnic.TCPInterfaceOptions{
			Address:     l.Addr().String(),
			Mux:         mux,
			ReadTimeout: *readTimeout,
		})
		opts = append(opts, vnic.WithInterface(tcpIface))
	case "tcp-dial":
		tcpIface = vnic.NewTCPInterface(logger, vnic.TCPInterfaceOptions{ReadTimeout: *readTimeout})
		opts = append(opts, vnic.WithInterface(tcpIface))
	}
	h, err := vnic.New(logger, profile, opts...)
	if err != nil {
		return err
	}
	kick(h.QOS().Class)
	fmt.Fprintf(os.Stderr, "self %x %x\n", h.ID().Bytes(nil), profile.Public)

	done := make(chan struct{})
	var doneOnce sync.Once
	closed := make(chan struct{})
	var closedOnce sync.Once
	h.AddPeerHandler(vnic.PeerHandlerFunc(func(p *vnic.Peer) {
		setupPeer(h, p, func() { doneOnce.Do(func() { close(done) }) })
		go func() {
			<-p.Done()
			closedOnce.Do(func() { close(closed) })
		}()
	}))

	switch mode {
	case "live":
		var l vnic.Link = &stdioLink{mtu: *mtu}
		x := ed25519util.KeyToCurve25519(profile)
		switch *aes {
		case "dial":
			peer, err := hex.DecodeString(*peerHex)
			if err != nil || len(peer) != 32 {
				return errors.New("-peer must be a 32 byte hex key")
			}
			if err := protoutil.WriteStream(l, &vnicv1.TCPMuxInit{ProtocolVersion: 1, PeerKey: peer}); err != nil {
				return err
			}
			var px [32]byte
			ed25519util.PublicKeyToCurve25519(&px, (*[32]byte)(peer))
			if l, err = handshakeAESLink(l, x, px[:]); err != nil {
				return err
			}
		case "accept":
			var init vnicv1.TCPMuxInit
			if err := protoutil.ReadStream(l, &init); err != nil {
				return err
			}
			if !bytes.Equal(init.PeerKey, profile.Public) {
				return errors.New("peer key not found")
			}
			if l, err = handshakeAESLink(l, x, nil); err != nil {
				return err
			}
		}
		// AddLink returns a nil error when peer init fails; check the peer.
		if p, _ := h.AddLink(l); p == nil {
			return errors.New("peer init failed")
		}
	case "tcp-listen":
		defer ln.Close()
		for {
			c, err := tcpIface.CreateLinkCandidate(context.Background(), h)
			if err != nil {
				return err
			}
			if d, _ := c.LocalDescription(); d != nil {
				fmt.Fprintf(os.Stderr, "uri %s\n", d.Description)
				break
			}
			time.Sleep(10 * time.Millisecond)
		}
	case "tcp-dial":
		// New runs Listen in a goroutine; it only sets the AES key here.
		if err := tcpIface.(vnic.Listener).Listen(h); err != nil {
			return err
		}
		if p, err := h.Dial(*uri); err != nil || p == nil {
			return fmt.Errorf("dial failed: %v", err)
		}
	}

	select {
	case <-done:
	case <-closed:
		select {
		case <-done:
		default:
			// For example the read deadline expired on an idle link.
			fmt.Fprintln(os.Stderr, "closed")
			return errors.New("peer closed before done")
		}
	case <-time.After(5 * time.Minute):
		return errors.New("timed out")
	}
	fmt.Fprintln(os.Stderr, "done")
	return nil
}

// setupPeer installs the script the Rust test drives:
//   - port 1000: echo each frame back on port 1001;
//   - port 2000: print each frame;
//   - ChannelPair(5000, 5001): reply to each message on port 5000 with its
//     bytes reversed, on port 5001;
//   - port 3000: send "go-3000-0".."go-3000-4", then 3000 bytes in one
//     Write and Flush (split at the MTU);
//   - port 9999: finish.
func setupPeer(h *vnic.Host, p *vnic.Peer, finish func()) {
	fmt.Fprintf(os.Stderr, "peer %x %x\n", p.HostID().Bytes(nil), p.Certificate.Key)
	qc := h.QOS().AddClass(1)

	echo := vnic.NewFrameWriter(p.Link, 1001, qc)
	p.SetHandler(1000, func(_ *vnic.Peer, f vnic.Frame) error {
		_, err := echo.WriteFrame(append([]byte(nil), f.Body...))
		return err
	})
	p.SetHandler(2000, func(_ *vnic.Peer, f vnic.Frame) error {
		fmt.Fprintf(os.Stderr, "recv 2000 %x\n", f.Body)
		return nil
	})
	p.SetHandler(9999, func(_ *vnic.Peer, f vnic.Frame) error {
		finish()
		return nil
	})

	_, rw1 := p.ChannelPair(5000, 5001, qc)
	go func() {
		buf := make([]byte, 1<<17)
		for {
			n, err := rw1.Read(buf)
			if err != nil {
				return
			}
			rev := make([]byte, n)
			for i := range rev {
				rev[i] = buf[n-1-i]
			}
			if _, err := rw1.Write(rev); err != nil {
				return
			}
			if err := rw1.Flush(); err != nil {
				return
			}
		}
	}()

	go func() {
		fw := vnic.NewFrameWriter(p.Link, 3000, qc)
		for i := 0; i < 5; i++ {
			if _, err := fw.WriteFrame([]byte(fmt.Sprintf("go-3000-%d", i))); err != nil {
				return
			}
		}
		if _, err := fw.Write(pattern(3000, 9)); err != nil {
			return
		}
		fw.Flush()
	}()
}
