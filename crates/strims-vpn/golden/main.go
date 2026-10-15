// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

// Golden-vector generator and live vpn peer for crates/strims-vpn.
//
// Run from the repo root:
//
//	go run -tags golden ./crates/strims-vpn/golden vectors > crates/strims-vpn/golden/vectors.txt
//	go run -tags golden ./crates/strims-vpn/golden live [-seed n] [-mtu n]
//
// "vectors" writes one "<kind> key=value ..." line per vector, with hex
// values:
//
//   - message: a message marshaled by three hosts in turn (Message.Marshal
//     with injected host keys); each hop's time is recorded because Marshal
//     reads the clock;
//   - verify: Go's Message.Verify for every hop of a tampered message;
//   - cipher: encryptMessage with an injected nonce, and decryptMessage on
//     the receiver;
//   - ht_record / pi_record and the service messages built from them.
//
// Regenerating changes the message and verify lines (their times and
// signatures); the other lines are stable.
//
// "live" runs two Go hosts, A and B, in one vpn network: A is linked to the
// Rust peer over stdin/stdout and to B over an in-memory pipe. Both run the
// script in setupNode and report on stderr:
//
//	node <a|b> <host id>
//	ready
//	recv <a|b> <src host id> <hops> <flags> <body hex>
//	peer <a|b> <peer host id>
//	done
//
// peer_exchange.go prints JSON to stdout (debug.PrintJSON), so os.Stdout is
// pointed at stderr and the link keeps the real stdout.
package main

import (
	"bytes"
	"context"
	"crypto/ed25519"
	"crypto/rand"
	"encoding/binary"
	"encoding/hex"
	"errors"
	"flag"
	"fmt"
	"io"
	"os"
	"reflect"
	"strconv"
	"strings"
	"sync"
	"time"
	"unsafe"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	vnicv1 "github.com/MemeLabs/strims/pkg/apis/vnic/v1"
	vpnv1 "github.com/MemeLabs/strims/pkg/apis/vpn/v1"
	"github.com/MemeLabs/strims/pkg/kademlia"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/MemeLabs/strims/pkg/vnic/qos"
	"github.com/MemeLabs/strims/pkg/vpn"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

//go:linkname encryptMessage github.com/MemeLabs/strims/pkg/vpn.encryptMessage
func encryptMessage(n *vpn.Network, m *vpn.Message, next func(*vpn.Network, *vpn.Message) error) error

//go:linkname decryptMessage github.com/MemeLabs/strims/pkg/vpn.decryptMessage
func decryptMessage(n *vpn.Network, m *vpn.Message, next func(*vpn.Network, *vpn.Message) error) error

//go:linkname hashTableRecordHash github.com/MemeLabs/strims/pkg/vpn.hashTableRecordHash
func hashTableRecordHash(key, salt []byte) []byte

func main() {
	if len(os.Args) < 2 {
		fail(errors.New("usage: golden vectors|live [flags]"))
	}
	var err error
	switch os.Args[1] {
	case "vectors":
		err = writeVectors(os.Stdout)
	case "live":
		err = runLive(os.Args[2:])
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

// setHostKey replaces a vnic host's random host key.
func setHostKey(h *vnic.Host, k *key.Key) {
	f := reflect.ValueOf(h).Elem().FieldByName("key")
	reflect.NewAt(f.Type(), unsafe.Pointer(f.UnsafeAddr())).Elem().Set(reflect.ValueOf(k))
}

// hostWithKey is a vnic host whose host key comes from seed.
func hostWithKey(seed byte) *vnic.Host {
	h, err := vnic.New(zap.NewNop(), keyFromSeed(seed^0xff))
	if err != nil {
		panic(err)
	}
	setHostKey(h, keyFromSeed(seed))
	return h
}

func seedHex(seed byte) string {
	return hex.EncodeToString(bytes.Repeat([]byte{seed}, 32))
}

func writeVectors(w io.Writer) error {
	if err := writeMessages(w); err != nil {
		return err
	}
	if err := writeCiphers(w); err != nil {
		return err
	}
	return writeRecords(w)
}

// --- messages ----------------------------------------------------------------

func writeMessages(w io.Writer) error {
	cases := []struct {
		dst      byte
		dstPort  uint16
		srcPort  uint16
		seq      uint16
		flags    uint16
		body     []byte
		hopSeeds []byte
	}{
		{0x01, 20000, 20001, 1, vpn.MstdFlags, []byte("test"), []byte{0x11, 0x22, 0x33}},
		{0x02, vnic.HashTablePort, vnic.HashTablePort, 0xfffe, vpn.Mbroadcast, pattern(300, 1), []byte{0x44, 0x55}},
		{0x03, 7, 9, 0x1234, vpn.Mbroadcast | vpn.Mnorelay, nil, []byte{0x66}},
		{0x04, 65535, 0, 42, 0, pattern(1000, 2), []byte{0x11, 0x22, 0x33, 0x44, 0x55}},
	}
	for _, c := range cases {
		dst, err := kademlia.UnmarshalID(keyFromSeed(c.dst).Public)
		if err != nil {
			return err
		}
		m := &vpn.Message{
			Header: vpn.MessageHeader{
				DstID:   dst,
				DstPort: c.dstPort,
				SrcPort: c.srcPort,
				Seq:     c.seq,
				Length:  uint16(len(c.body)),
				Flags:   c.flags,
			},
			Body: c.body,
		}
		var wires, times, seeds []string
		var last []byte
		for i, s := range c.hopSeeds {
			h := hostWithKey(s)
			b := make([]byte, m.Size())
			n, err := m.Marshal(b, h)
			if err != nil {
				return err
			}
			b = b[:n]
			last = b
			wires = append(wires, hex.EncodeToString(b))
			seeds = append(seeds, seedHex(s))
			off := len(b) - 104
			times = append(times, strconv.FormatInt(int64(binary.BigEndian.Uint64(b[off:])), 10))
			m = &vpn.Message{}
			if _, err := m.Unmarshal(b); err != nil {
				return err
			}
			if m.Trailer.Hops != i+1 {
				return errors.New("unexpected hop count")
			}
			for j := 0; j <= i; j++ {
				if !m.Verify(j) {
					return errors.New("go failed to verify its own message")
				}
			}
		}
		id := m.ID()
		fmt.Fprintf(w, "message dst_seed=%s dst_port=%d src_port=%d seq=%d flags=%d body=%x hop_seeds=%s times=%s id=%x wires=%s\n",
			seedHex(c.dst), c.dstPort, c.srcPort, c.seq, c.flags, c.body,
			strings.Join(seeds, ","), strings.Join(times, ","), id[:], strings.Join(wires, ","))

		// Go's Verify on tampered copies of the last wire.
		hops := len(c.hopSeeds)
		bodyEnd := 42 + len(c.body)
		positions := []int{0, 33, bodyEnd - 1, bodyEnd, bodyEnd + 39, bodyEnd + 40, bodyEnd + 103}
		if hops > 1 {
			positions = append(positions, bodyEnd+104, bodyEnd+104+50, len(last)-1)
		}
		for _, pos := range positions {
			if pos < 0 || pos >= len(last) {
				continue
			}
			t := append([]byte(nil), last...)
			t[pos] ^= 0x01
			var tm vpn.Message
			if _, err := tm.Unmarshal(t); err != nil {
				fmt.Fprintf(w, "verify wire=%x pos=%d decode=error\n", last, pos)
				continue
			}
			var res []string
			for j := 0; j <= tm.Trailer.Hops; j++ {
				res = append(res, strconv.FormatBool(tm.Verify(j)))
			}
			fmt.Fprintf(w, "verify wire=%x pos=%d hops=%d results=%s\n", last, pos, tm.Trailer.Hops, strings.Join(res, ","))
		}
	}
	return nil
}

// --- ciphers -----------------------------------------------------------------

// nonceReader hands out queued nonces to crypto/rand.Read.
type nonceReader struct {
	mu     sync.Mutex
	nonces [][]byte
}

func (r *nonceReader) Read(b []byte) (int, error) {
	r.mu.Lock()
	defer r.mu.Unlock()
	if len(r.nonces) == 0 {
		return 0, errors.New("no nonce queued")
	}
	n := copy(b, r.nonces[0])
	r.nonces = r.nonces[1:]
	return n, nil
}

func networkFor(h *vnic.Host) (*vpn.Network, error) {
	v, err := vpn.New(zap.NewNop(), h)
	if err != nil {
		return nil, err
	}
	node, err := v.AddNetwork(keyFromSeed(0xee).Public)
	if err != nil {
		return nil, err
	}
	return node.Network, nil
}

func writeCiphers(w io.Writer) error {
	saved := rand.Reader
	defer func() { rand.Reader = saved }()

	cases := []struct {
		a, b  byte
		plain []byte
	}{
		{0x11, 0x22, []byte("some test string")},
		{0x22, 0x11, []byte{}},
		{0x33, 0x44, pattern(1000, 3)},
		{0x55, 0x55, []byte("to self")},
	}
	for i, c := range cases {
		nonce := pattern(12, byte(0xa0+i))

		na, err := networkFor(hostWithKey(c.a))
		if err != nil {
			return err
		}
		nb, err := networkFor(hostWithKey(c.b))
		if err != nil {
			return err
		}
		idA, _ := kademlia.UnmarshalID(keyFromSeed(c.a).Public)
		idB, _ := kademlia.UnmarshalID(keyFromSeed(c.b).Public)

		var sealed []byte
		// After the hosts: vnic.New draws its host key from crypto/rand.
		rand.Reader = &nonceReader{nonces: [][]byte{nonce}}
		m := &vpn.Message{
			Header: vpn.MessageHeader{DstID: idB, Length: uint16(len(c.plain))},
			Body:   append([]byte(nil), c.plain...),
		}
		err = encryptMessage(na, m, func(_ *vpn.Network, m *vpn.Message) error {
			sealed = append([]byte(nil), m.Body...)
			if int(m.Header.Length) != len(m.Body) {
				return errors.New("length not updated")
			}
			return nil
		})
		rand.Reader = saved
		if err != nil {
			return err
		}
		if !bytes.Equal(sealed[:12], nonce) {
			return errors.New("injected nonce not used")
		}

		var opened []byte
		r := &vpn.Message{
			Header:  vpn.MessageHeader{DstID: idB, Length: uint16(len(sealed))},
			Body:    sealed,
			Trailer: vpn.MessageTrailer{Hops: 1, Entries: []vpn.MessageTrailerEntry{{HostID: idA}}},
		}
		err = decryptMessage(nb, r, func(_ *vpn.Network, m *vpn.Message) error {
			opened = append([]byte(nil), m.Body...)
			return nil
		})
		if err != nil {
			return err
		}
		if !bytes.Equal(opened, c.plain) {
			return errors.New("go round trip mismatch")
		}
		fmt.Fprintf(w, "cipher a_seed=%s b_seed=%s nonce=%x plain=%x sealed=%x\n",
			seedHex(c.a), seedHex(c.b), nonce, c.plain, sealed)
	}
	return nil
}

// --- records -----------------------------------------------------------------

func marshal(m proto.Message) []byte {
	b, err := proto.Marshal(m)
	if err != nil {
		panic(err)
	}
	return b
}

func writeRecords(w io.Writer) error {
	for _, c := range []struct {
		seed      byte
		salt      []byte
		value     []byte
		timestamp int64
	}{
		{0x71, []byte("foo"), []byte("bar"), 1700000000},
		{0x72, nil, pattern(200, 5), 1},
		{0x73, pattern(40, 6), nil, 0},
	} {
		k := keyFromSeed(c.seed)
		r := &vpnv1.HashTableMessage_Record{
			Salt:      c.salt,
			Value:     c.value,
			Timestamp: c.timestamp,
		}
		if err := dao.SignMessage(r, k); err != nil {
			return err
		}
		hash := hashTableRecordHash(k.Public, c.salt)
		publish := &vpnv1.HashTableMessage{Body: &vpnv1.HashTableMessage_Publish_{Publish: &vpnv1.HashTableMessage_Publish{Record: r}}}
		unpublish := &vpnv1.HashTableMessage{Body: &vpnv1.HashTableMessage_Unpublish_{Unpublish: &vpnv1.HashTableMessage_Unpublish{Record: r}}}
		get := &vpnv1.HashTableMessage{Body: &vpnv1.HashTableMessage_GetRequest_{GetRequest: &vpnv1.HashTableMessage_GetRequest{
			RequestId: 0x0123456789abcdef, Hash: hash, IfModifiedSince: c.timestamp - 1,
		}}}
		res := &vpnv1.HashTableMessage{Body: &vpnv1.HashTableMessage_GetResponse_{GetResponse: &vpnv1.HashTableMessage_GetResponse{
			RequestId: 0xfedcba9876543210, Record: r,
		}}}
		fmt.Fprintf(w, "ht_record seed=%s salt=%x value=%x timestamp=%d hash=%x record=%x publish=%x unpublish=%x get_request=%x get_response=%x\n",
			seedHex(c.seed), c.salt, c.value, c.timestamp, hash, marshal(r),
			marshal(publish), marshal(unpublish), marshal(get), marshal(res))
	}

	var records []*vpnv1.PeerIndexMessage_Record
	for _, c := range []struct {
		seed      byte
		key       []byte
		salt      []byte
		port      uint32
		timestamp int64
	}{
		{0x81, []byte("swarm"), []byte("salt"), 0, 1700000000},
		{0x82, pattern(32, 7), nil, 65535, 1700000600},
	} {
		h := hostWithKey(c.seed)
		hash := hashTableRecordHash(c.key, c.salt)
		r := &vpnv1.PeerIndexMessage_Record{
			Hash:      hash,
			HostId:    h.ID().Bytes(nil),
			Port:      c.port,
			Timestamp: c.timestamp,
		}
		if err := dao.SignMessage(r, h.Key()); err != nil {
			return err
		}
		records = append(records, r)
		publish := &vpnv1.PeerIndexMessage{Body: &vpnv1.PeerIndexMessage_Publish_{Publish: &vpnv1.PeerIndexMessage_Publish{Record: r}}}
		unpublish := &vpnv1.PeerIndexMessage{Body: &vpnv1.PeerIndexMessage_Unpublish_{Unpublish: &vpnv1.PeerIndexMessage_Unpublish{Record: r}}}
		search := &vpnv1.PeerIndexMessage{Body: &vpnv1.PeerIndexMessage_SearchRequest_{SearchRequest: &vpnv1.PeerIndexMessage_SearchRequest{
			RequestId: 99, Hash: hash,
		}}}
		fmt.Fprintf(w, "pi_record seed=%s key=%x salt=%x port=%d timestamp=%d record=%x publish=%x unpublish=%x search_request=%x\n",
			seedHex(c.seed), c.key, c.salt, c.port, c.timestamp, marshal(r), marshal(publish), marshal(unpublish), marshal(search))
	}
	res := &vpnv1.PeerIndexMessage{Body: &vpnv1.PeerIndexMessage_SearchResponse_{SearchResponse: &vpnv1.PeerIndexMessage_SearchResponse{
		RequestId: 99, Records: records,
	}}}
	fmt.Fprintf(w, "pi_search_response response=%x\n", marshal(res))

	descs := []*vnicv1.LinkDescription{
		{Interface: "tcp", Description: "tcp://127.0.0.1:8080/abcd"},
		{Interface: "webrtc", Description: `{"type":"offer","sdp":"v=0\r\n"}`},
	}
	offer := &vpnv1.PeerExchangeMessage{Body: &vpnv1.PeerExchangeMessage_LinkOffer_{LinkOffer: &vpnv1.PeerExchangeMessage_LinkOffer{
		ExchangeId: 0x1122334455667788, Descriptions: descs,
	}}}
	answer := &vpnv1.PeerExchangeMessage{Body: &vpnv1.PeerExchangeMessage_LinkAnswer_{LinkAnswer: &vpnv1.PeerExchangeMessage_LinkAnswer{
		ExchangeId: 0x1122334455667788, Descriptions: descs[1:],
	}}}
	reject := &vpnv1.PeerExchangeMessage{Body: &vpnv1.PeerExchangeMessage_LinkAnswer_{LinkAnswer: &vpnv1.PeerExchangeMessage_LinkAnswer{
		ExchangeId: 7, ErrorMessage: "unable to add new peers",
	}}}
	fmt.Fprintf(w, "px offer=%x answer=%x reject=%x\n", marshal(offer), marshal(answer), marshal(reject))
	return nil
}

// --- live peer ---------------------------------------------------------------

// kick works around qos.Control's lost wakeups (see crates/strims-vnic's
// golden helper): a periodic no-op enqueue bounds the stall.
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

type stdioLink struct {
	in  *os.File
	out *os.File
	mtu int
}

func (l *stdioLink) Read(p []byte) (int, error)  { return l.in.Read(p) }
func (l *stdioLink) Write(p []byte) (int, error) { return l.out.Write(p) }
func (l *stdioLink) Close() error                { return nil }
func (l *stdioLink) MTU() int                    { return l.mtu }

// pipe is a buffered in-memory link between two Go hosts.
type pipeBuf struct {
	mu   sync.Mutex
	cond *sync.Cond
	buf  bytes.Buffer
}

func newPipeBuf() *pipeBuf {
	p := &pipeBuf{}
	p.cond = sync.NewCond(&p.mu)
	return p
}

type pipeEnd struct {
	r, w *pipeBuf
	mtu  int
}

func newPipe(mtu int) (*pipeEnd, *pipeEnd) {
	a, b := newPipeBuf(), newPipeBuf()
	return &pipeEnd{a, b, mtu}, &pipeEnd{b, a, mtu}
}

func (p *pipeEnd) Read(b []byte) (int, error) {
	p.r.mu.Lock()
	defer p.r.mu.Unlock()
	for p.r.buf.Len() == 0 {
		p.r.cond.Wait()
	}
	return p.r.buf.Read(b)
}

func (p *pipeEnd) Write(b []byte) (int, error) {
	p.w.mu.Lock()
	defer p.w.mu.Unlock()
	n, err := p.w.buf.Write(b)
	p.w.cond.Broadcast()
	return n, err
}

func (p *pipeEnd) Close() error { return nil }
func (p *pipeEnd) MTU() int     { return p.mtu }

var stderrMu sync.Mutex

func report(format string, args ...any) {
	stderrMu.Lock()
	defer stderrMu.Unlock()
	fmt.Fprintf(os.Stderr, format+"\n", args...)
}

const (
	vpnPort     = 10000
	appPort     = 20000
	controlPort = 30000
	replyPort   = 30001
)

type liveNode struct {
	name string
	vnic *vnic.Host
	node *vpn.Node
	done chan struct{}
	once sync.Once
}

func newLiveNode(name string, seed byte, opts ...vnic.HostOption) (*liveNode, error) {
	logger := zap.NewNop()
	h, err := vnic.New(logger, keyFromSeed(seed), append(opts, vnic.WithLabel("go-"+name))...)
	if err != nil {
		return nil, err
	}
	kick(h.QOS().Class)
	v, err := vpn.New(logger, h)
	if err != nil {
		return nil, err
	}
	node, err := v.AddNetwork(keyFromSeed(0xee).Public)
	if err != nil {
		return nil, err
	}
	n := &liveNode{name: name, vnic: h, node: node, done: make(chan struct{})}
	h.AddPeerHandler(vnic.PeerHandlerFunc(func(p *vnic.Peer) {
		report("peer %s %x", name, p.HostID().Bytes(nil))
		node.Network.AddPeer(p, vpnPort, vpnPort)
	}))
	n.setup()
	return n, nil
}

func (n *liveNode) reply(to kademlia.ID, port uint16, s string) {
	if err := n.node.Network.Send(to, port, controlPort, []byte(s)); err != nil {
		report("error %s reply: %v", n.name, err)
	}
}

// setup installs the script the Rust test drives:
//   - port 20000: report every message; answer "ping X" with "pong X" and
//     a broadcast with "ack <body>", both unicast and encrypted;
//   - port 30000: commands from the Rust peer, answered on its source port.
func (n *liveNode) setup() {
	net := n.node.Network
	net.SetHandler(appPort, vpn.MessageHandlerFunc(func(m *vpn.Message) error {
		body := append([]byte(nil), m.Body...)
		src := m.SrcHostID()
		report("recv %s %x %d %d %x", n.name, src.Bytes(nil), m.Trailer.Hops, m.Header.Flags, body)
		if src.Equals(n.vnic.ID()) {
			return nil
		}
		go func() {
			switch {
			case bytes.HasPrefix(body, []byte("ping ")):
				n.send(src, appPort, "pong "+string(body[5:]))
			case m.Header.Flags&vpn.Mbroadcast != 0:
				n.send(src, appPort, "ack "+string(body))
			}
		}()
		return nil
	}))
	net.SetHandler(controlPort, vpn.MessageHandlerFunc(func(m *vpn.Message) error {
		cmd := strings.Fields(string(m.Body))
		src, port := m.SrcHostID(), m.Header.SrcPort
		go n.command(cmd, src, port)
		return nil
	}))
}

func (n *liveNode) send(to kademlia.ID, port uint16, s string) {
	if err := n.node.Network.Send(to, port, appPort, []byte(s)); err != nil {
		report("error %s send: %v", n.name, err)
	}
}

func unhex(s string) []byte {
	b, err := hex.DecodeString(s)
	if err != nil {
		panic(err)
	}
	return b
}

func (n *liveNode) command(cmd []string, src kademlia.ID, port uint16) {
	if len(cmd) == 0 {
		return
	}
	ctx := context.Background()
	report("cmd %s %s", n.name, strings.Join(cmd, " "))
	switch cmd[0] {
	case "bcast":
		if err := n.node.Network.Broadcast(appPort, appPort, []byte("go-bcast "+n.name+" "+cmd[1])); err != nil {
			n.reply(src, port, "error "+err.Error())
			return
		}
		n.reply(src, port, "bcast-ok")
	case "ht-set":
		if _, err := n.node.HashTable.Set(ctx, keyFromSeed(0x71), unhex(cmd[1]), unhex(cmd[2])); err != nil {
			n.reply(src, port, "error "+err.Error())
			return
		}
		n.reply(src, port, "ht-set-ok")
	case "ht-get":
		ch, err := n.node.HashTable.Get(ctx, unhex(cmd[1]), unhex(cmd[2]), vpn.DisableCache(), vpn.WithTimeout(20*time.Second))
		if err != nil {
			n.reply(src, port, "error "+err.Error())
			return
		}
		if v, ok := <-ch; ok {
			n.reply(src, port, "ht-value "+hex.EncodeToString(v))
		} else {
			n.reply(src, port, "ht-none")
		}
	case "pi-publish":
		p, _ := strconv.Atoi(cmd[3])
		if err := n.node.PeerIndex.Publish(ctx, unhex(cmd[1]), unhex(cmd[2]), uint16(p)); err != nil {
			n.reply(src, port, "error "+err.Error())
			return
		}
		// The publish runs in a goroutine; give it time to go out.
		time.Sleep(200 * time.Millisecond)
		n.reply(src, port, "pi-publish-ok")
	case "pi-search":
		ch, err := n.node.PeerIndex.Search(ctx, unhex(cmd[1]), unhex(cmd[2]))
		if err != nil {
			n.reply(src, port, "error "+err.Error())
			return
		}
		select {
		case h, ok := <-ch:
			if ok {
				n.reply(src, port, fmt.Sprintf("pi-host %x %d", h.HostID.Bytes(nil), h.Port))
				return
			}
		case <-time.After(20 * time.Second):
		}
		n.reply(src, port, "pi-none")
	case "px-connect":
		id, err := kademlia.UnmarshalID(unhex(cmd[1]))
		if err == nil {
			err = n.node.PeerExchange.Connect(id)
		}
		if err != nil {
			n.reply(src, port, "error "+err.Error())
			return
		}
		n.reply(src, port, "px-connect-ok")
	case "done":
		n.once.Do(func() { close(n.done) })
	}
}

func runLive(args []string) error {
	fs := flag.NewFlagSet("live", flag.ExitOnError)
	seed := fs.Int("seed", 0x42, "profile key seed byte of host A")
	mtu := fs.Int("mtu", 16*1024, "stdio link mtu")
	if err := fs.Parse(args); err != nil {
		return err
	}
	// peer_exchange.go's debug.PrintJSON writes to os.Stdout.
	link := &stdioLink{in: os.Stdin, out: os.Stdout, mtu: *mtu}
	os.Stdout = os.Stderr

	a, err := newLiveNode("a", byte(*seed))
	if err != nil {
		return err
	}
	// B can make link candidates (a TCP interface without a listener) so
	// that it can dial what a peer exchange answer advertises.
	tcp := vnic.NewTCPInterface(zap.NewNop(), vnic.TCPInterfaceOptions{})
	b, err := newLiveNode("b", byte(*seed)+1, vnic.WithInterface(tcp))
	if err != nil {
		return err
	}
	// vnic.New runs Listen in a goroutine; it only sets the AES key here.
	if err := tcp.(vnic.Listener).Listen(b.vnic); err != nil {
		return err
	}
	report("node a %x", a.vnic.ID().Bytes(nil))
	report("node b %x", b.vnic.ID().Bytes(nil))

	pa, pb := newPipe(16 * 1024)
	var wg sync.WaitGroup
	wg.Add(2)
	var errA, errB error
	go func() {
		defer wg.Done()
		_, errA = a.vnic.AddLink(pa)
	}()
	go func() {
		defer wg.Done()
		_, errB = b.vnic.AddLink(pb)
	}()
	wg.Wait()
	if errA != nil || errB != nil {
		return fmt.Errorf("a-b link: %v %v", errA, errB)
	}
	report("ready")

	go func() {
		// AddLink returns a nil error when peer init fails: check the peer.
		if p, _ := a.vnic.AddLink(link); p == nil {
			fail(errors.New("peer init with the rust peer failed"))
		}
	}()

	for _, n := range []*liveNode{a, b} {
		select {
		case <-n.done:
		case <-time.After(5 * time.Minute):
			return errors.New("timed out")
		}
	}
	report("done")
	_ = timeutil.Now
	return nil
}
