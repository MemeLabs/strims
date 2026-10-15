// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

// A Go node for the live Go<->Rust node tests in
// crates/strims-node/tests/interop_go/.
//
// It builds the stack cmd/svc's newVPN builds, over real localhost sockets:
//
//   - vnic.New with a TCP interface on a TCP mux, and a WebSocket interface
//     (AllowInsecure) mounted on an HTTP mux; with -listen=false both can
//     only dial and advertise nothing (a browser-like node);
//   - with -webrtc, also the WebRTC interface: NewWebRTCInterface on a
//     NewWebRTCUDPMux bound to 127.0.0.1 with no ICE servers (cmd/svc's
//     UDPMuxAddress), after TCP and WebSocket as cmd/svc orders them;
//   - vpn.New with one network, every vnic peer bound to it on a fixed port
//     pair (Network.AddPeer(peer, port, port)), the same pairing as the
//     Rust node's auto_bind_port;
//   - a transfer-like ppspp wiring (internal/transfer/control.go AddPeer):
//     a ppspp.Runner, and per peer a FrameWriter on TransferPort in its own
//     QoS class, RunPeer, and a TransferPort handler. Swarms run with fixed
//     channel ids through RunnerPeer.RunSwarm.
//
// It is controlled over stdin with one command per line,
//
//	<id> <verb> <args...>
//
// and answers on stdout with "ok <id> <values...>" or "err <id> <message>".
// Events go to stdout too:
//
//	ready <host id> <tcp uri or -> <ws uri or -> <webrtc udp addr or ->
//	peer <host id> <platform hex> <version hex> <link type>
//	peer-removed <host id>
//	msg <src host id> <src port> <dst port> <hops> <flags> <body hex>
//	transfer-error <host id> <message hex>
//	written <swarm> <bytes>
//	read-done <swarm> <start> <length> <sha256 hex> <mismatches>
//	read-error <swarm> <offset> <message hex>
//
// Byte strings are hex. Diagnostics go to stderr. Stream bytes are
// streamByte(absolute offset), the same function the Rust test uses.
//
// Constraints the helper keeps:
//
//   - Only the protocol writes to the real stdout; os.Stdout points at
//     stderr for everything else.
//   - A queued QoS packet waits at most 2 ms for the run loop (kick).
//   - A dial succeeds only with a peer.
//   - Every new peer is bound to the network, replacing an older link to
//     the same host, and a peer's exit unbinds the host only while that
//     peer is still current.
package main

import (
	"bufio"
	"context"
	"crypto/ed25519"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"flag"
	"fmt"
	"net"
	"net/http"
	"os"
	"reflect"
	"strconv"
	"strings"
	"sync"
	"time"
	"unsafe"

	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"github.com/MemeLabs/strims/pkg/httputil"
	"github.com/MemeLabs/strims/pkg/kademlia"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/ppspp/codec"
	"github.com/MemeLabs/strims/pkg/vnic"
	"github.com/MemeLabs/strims/pkg/vnic/qos"
	"github.com/MemeLabs/strims/pkg/vpn"
	"go.uber.org/zap"
)

func streamByte(o uint64) byte {
	x := (o * 0x9e3779b97f4a7c15) ^ (o >> 7)
	return byte(x >> 56)
}

func keyFromSeed(seed []byte) *key.Key {
	priv := ed25519.NewKeyFromSeed(seed)
	return &key.Key{
		Type:    key.KeyType_KEY_TYPE_ED25519,
		Private: priv,
		Public:  priv.Public().(ed25519.PublicKey),
	}
}

// linkType names the transport under a peer's instrumented link:
// *vnic.aesLink (TCP), *httputil.WSReadWriter (WebSocket) or *vnic.dcLink
// (a WebRTC data channel).
func linkType(l vnic.Link) string {
	v := reflect.ValueOf(l)
	if v.Kind() == reflect.Ptr && v.Elem().Kind() == reflect.Struct {
		if inner := v.Elem().FieldByName("Link"); inner.IsValid() && !inner.IsNil() {
			return inner.Elem().Type().String()
		}
	}
	return v.Type().String()
}

// setHostKey replaces a vnic host's random host key, so host ids (and so
// vpn routes) are the same in every run.
func setHostKey(h *vnic.Host, k *key.Key) {
	f := reflect.ValueOf(h).Elem().FieldByName("key")
	reflect.NewAt(f.Type(), unsafe.Pointer(f.UnsafeAddr())).Elem().Set(reflect.ValueOf(k))
}

// kick enqueues a no-op packet on its own session every 2 ms, so a packet
// queued while the QoS run loop isn't waiting is sent within that time.
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

// --- output ------------------------------------------------------------------

var (
	outMu sync.Mutex
	out   *os.File
)

func emit(format string, args ...any) {
	outMu.Lock()
	defer outMu.Unlock()
	fmt.Fprintf(out, format+"\n", args...)
}

func logf(format string, args ...any) {
	fmt.Fprintf(os.Stderr, format+"\n", args...)
}

func hexs(s string) string {
	if s == "" {
		return "-"
	}
	return hex.EncodeToString([]byte(s))
}

func unhex(s string) ([]byte, error) {
	if s == "-" {
		return nil, nil
	}
	return hex.DecodeString(s)
}

func idHex(id kademlia.ID) string {
	return hex.EncodeToString(id.Bytes(nil))
}

// --- node --------------------------------------------------------------------

type peerState struct {
	peer *vnic.Peer
	rp   *ppspp.RunnerPeer
}

type swarmState struct {
	swarm  *ppspp.Swarm
	writer *ppspp.Writer
}

type node struct {
	host        *vnic.Host
	vpn         *vpn.Node
	vpnPort     uint16
	runner      *ppspp.Runner
	transferQOS *qos.Class

	mu     sync.Mutex
	peers  map[kademlia.ID]*peerState
	swarms map[string]*swarmState
}

func (n *node) handlePeer(p *vnic.Peer) {
	id := p.HostID()
	w := vnic.NewFrameWriter(p.Link, vnic.TransferPort, n.transferQOS)
	cr, rp := n.runner.RunPeer(id.Bytes(nil), w)
	p.SetHandler(vnic.TransferPort, func(_ *vnic.Peer, f vnic.Frame) error {
		err := cr.HandleMessage(f.Body)
		if err != nil {
			emit("transfer-error %s %s", idHex(id), hexs(err.Error()))
		}
		return err
	})

	ps := &peerState{peer: p, rp: rp}
	n.mu.Lock()
	n.peers[id] = ps
	n.mu.Unlock()

	// The host may still be bound through a replaced peer's link.
	n.vpn.Network.RemovePeer(id)
	n.vpn.Network.AddPeer(p, n.vpnPort, n.vpnPort)
	emit("peer %s %s %s %s", idHex(id), hexs(p.NodePlatform), hexs(p.NodeVersion), linkType(p.Link))

	go func() {
		<-p.Done()
		rp.Stop()
		w.Close()
		n.mu.Lock()
		current := n.peers[id] == ps
		if current {
			delete(n.peers, id)
		}
		n.mu.Unlock()
		if current {
			n.vpn.Network.RemovePeer(id)
		}
		emit("peer-removed %s", idHex(id))
	}()
}

func (n *node) peer(hexID string) (*peerState, error) {
	b, err := hex.DecodeString(hexID)
	if err != nil {
		return nil, err
	}
	id, err := kademlia.UnmarshalID(b)
	if err != nil {
		return nil, err
	}
	n.mu.Lock()
	defer n.mu.Unlock()
	ps, ok := n.peers[id]
	if !ok {
		return nil, errors.New("unknown peer")
	}
	return ps, nil
}

func (n *node) swarm(name string) (*swarmState, error) {
	n.mu.Lock()
	defer n.mu.Unlock()
	s, ok := n.swarms[name]
	if !ok {
		return nil, errors.New("unknown swarm")
	}
	return s, nil
}

func parseID(s string) (kademlia.ID, error) {
	b, err := hex.DecodeString(s)
	if err != nil {
		return kademlia.ID{}, err
	}
	return kademlia.UnmarshalID(b)
}

func parseUint(s string, bits int) (uint64, error) {
	return strconv.ParseUint(s, 10, bits)
}

func swarmOptions(liveWindow, mode string) (ppspp.SwarmOptions, error) {
	lw, err := strconv.Atoi(liveWindow)
	if err != nil {
		return ppspp.SwarmOptions{}, err
	}
	o := ppspp.SwarmOptions{LiveWindow: lw}
	switch mode {
	case "mandatory":
		o.DeliveryMode = ppspp.MandatoryDeliveryMode
	case "low-latency":
		o.DeliveryMode = ppspp.LowLatencyDeliveryMode
	case "best-effort":
		o.DeliveryMode = ppspp.BestEffortDeliveryMode
	default:
		return o, fmt.Errorf("unknown delivery mode %q", mode)
	}
	return o, nil
}

// command runs one command and returns the reply values.
func (n *node) command(verb string, args []string) (string, error) {
	need := func(k int) error {
		if len(args) < k {
			return fmt.Errorf("%s: want %d args, got %d", verb, k, len(args))
		}
		return nil
	}
	ctx := context.Background()
	network := n.vpn.Network

	switch verb {
	case "dial":
		if err := need(1); err != nil {
			return "", err
		}
		p, err := n.host.Dial(args[0])
		if err != nil {
			return "", err
		}
		// A nil peer means peer init failed.
		if p == nil {
			return "", errors.New("peer init failed")
		}
		return idHex(p.HostID()), nil

	case "peers":
		var ids []string
		for _, p := range n.host.Peers() {
			ids = append(ids, idHex(p.HostID()))
		}
		if len(ids) == 0 {
			return "-", nil
		}
		return strings.Join(ids, ","), nil

	case "close-peer":
		if err := need(1); err != nil {
			return "", err
		}
		ps, err := n.peer(args[0])
		if err != nil {
			return "", err
		}
		ps.peer.Close()
		return "", nil

	case "max-peers":
		if err := need(1); err != nil {
			return "", err
		}
		v, err := strconv.Atoi(args[0])
		if err != nil {
			return "", err
		}
		n.host.SetMaxPeers(v)
		return "", nil

	case "rate-limit":
		if err := need(1); err != nil {
			return "", err
		}
		v, err := parseUint(args[0], 64)
		if err != nil {
			return "", err
		}
		n.host.QOS().SetRateLimit(v)
		return "", nil

	case "handler":
		if err := need(1); err != nil {
			return "", err
		}
		port, err := parseUint(args[0], 16)
		if err != nil {
			return "", err
		}
		return "", network.SetHandler(uint16(port), vpn.MessageHandlerFunc(func(m *vpn.Message) error {
			emit("msg %s %d %d %d %d %s", idHex(m.SrcHostID()), m.Header.SrcPort, m.Header.DstPort, m.Trailer.Hops, m.Header.Flags, hex.EncodeToString(m.Body))
			return nil
		}))

	case "send", "bcast":
		// send <dst> <dst port> <src port> <body>; bcast <dst port> <src port> <body>
		if verb == "bcast" {
			args = append([]string{"-"}, args...)
		}
		if err := need(4); err != nil {
			return "", err
		}
		dp, err := parseUint(args[1], 16)
		if err != nil {
			return "", err
		}
		sp, err := parseUint(args[2], 16)
		if err != nil {
			return "", err
		}
		body, err := unhex(args[3])
		if err != nil {
			return "", err
		}
		if verb == "bcast" {
			return "", network.Broadcast(uint16(dp), uint16(sp), body)
		}
		dst, err := parseID(args[0])
		if err != nil {
			return "", err
		}
		return "", network.Send(dst, uint16(dp), uint16(sp), body)

	case "ht-set":
		// ht-set <key seed> <salt> <value>
		if err := need(3); err != nil {
			return "", err
		}
		seed, err := unhex(args[0])
		if err != nil {
			return "", err
		}
		if len(seed) != ed25519.SeedSize {
			return "", errors.New("bad key seed")
		}
		salt, err := unhex(args[1])
		if err != nil {
			return "", err
		}
		value, err := unhex(args[2])
		if err != nil {
			return "", err
		}
		_, err = n.vpn.HashTable.Set(ctx, keyFromSeed(seed), salt, value)
		return "", err

	case "ht-get":
		// ht-get <key> <salt> <timeout ms>: the first value, or "-"
		if err := need(3); err != nil {
			return "", err
		}
		k, err := unhex(args[0])
		if err != nil {
			return "", err
		}
		salt, err := unhex(args[1])
		if err != nil {
			return "", err
		}
		ms, err := parseUint(args[2], 32)
		if err != nil {
			return "", err
		}
		ch, err := n.vpn.HashTable.Get(ctx, k, salt, vpn.DisableCache(), vpn.WithTimeout(time.Duration(ms)*time.Millisecond))
		if err != nil {
			return "", err
		}
		if v, ok := <-ch; ok {
			go func() {
				for range ch {
				}
			}()
			return hex.EncodeToString(v), nil
		}
		return "-", nil

	case "pi-publish":
		// pi-publish <key> <salt> <port>
		if err := need(3); err != nil {
			return "", err
		}
		k, err := unhex(args[0])
		if err != nil {
			return "", err
		}
		salt, err := unhex(args[1])
		if err != nil {
			return "", err
		}
		port, err := parseUint(args[2], 16)
		if err != nil {
			return "", err
		}
		return "", n.vpn.PeerIndex.Publish(ctx, k, salt, uint16(port))

	case "pi-search":
		// pi-search <key> <salt> <timeout ms> <want host>: every host:port
		// found until <want host> is or the timeout passes
		if err := need(4); err != nil {
			return "", err
		}
		k, err := unhex(args[0])
		if err != nil {
			return "", err
		}
		salt, err := unhex(args[1])
		if err != nil {
			return "", err
		}
		ms, err := parseUint(args[2], 32)
		if err != nil {
			return "", err
		}
		sctx, cancel := context.WithTimeout(ctx, time.Duration(ms)*time.Millisecond)
		defer cancel()
		ch, err := n.vpn.PeerIndex.Search(sctx, k, salt)
		if err != nil {
			return "", err
		}
		var found []string
		for {
			select {
			case h, ok := <-ch:
				if !ok {
					return strings.Join(append(found, "-"), ","), nil
				}
				s := fmt.Sprintf("%s:%d", idHex(h.HostID), h.Port)
				found = append(found, s)
				if idHex(h.HostID) == args[3] {
					go func() {
						for range ch {
						}
					}()
					return strings.Join(found, ","), nil
				}
			case <-sctx.Done():
				go func() {
					for range ch {
					}
				}()
				return strings.Join(append(found, "-"), ","), nil
			}
		}

	case "px-connect":
		if err := need(1); err != nil {
			return "", err
		}
		id, err := parseID(args[0])
		if err != nil {
			return "", err
		}
		return "", n.vpn.PeerExchange.Connect(id)

	case "swarm-create":
		// swarm-create <name> <key seed> <live window> <mode>: the swarm id
		if err := need(4); err != nil {
			return "", err
		}
		seed, err := unhex(args[1])
		if err != nil {
			return "", err
		}
		if len(seed) != ed25519.SeedSize {
			return "", errors.New("bad key seed")
		}
		o, err := swarmOptions(args[2], args[3])
		if err != nil {
			return "", err
		}
		w, err := ppspp.NewWriter(ppspp.WriterOptions{SwarmOptions: o, Key: keyFromSeed(seed)})
		if err != nil {
			return "", err
		}
		n.mu.Lock()
		n.swarms[args[0]] = &swarmState{swarm: w.Swarm(), writer: w}
		n.mu.Unlock()
		return w.Swarm().ID().String(), nil

	case "swarm-add":
		// swarm-add <name> <public key> <live window> <mode>: the swarm id
		if err := need(4); err != nil {
			return "", err
		}
		pub, err := unhex(args[1])
		if err != nil {
			return "", err
		}
		o, err := swarmOptions(args[2], args[3])
		if err != nil {
			return "", err
		}
		s, err := ppspp.NewSwarm(ppspp.NewSwarmID(pub), o)
		if err != nil {
			return "", err
		}
		n.mu.Lock()
		n.swarms[args[0]] = &swarmState{swarm: s}
		n.mu.Unlock()
		return s.ID().String(), nil

	case "swarm-run", "swarm-stop":
		// swarm-run <name> <peer> <channel> <peer channel>; swarm-stop <name> <peer>
		if err := need(2); err != nil {
			return "", err
		}
		s, err := n.swarm(args[0])
		if err != nil {
			return "", err
		}
		ps, err := n.peer(args[1])
		if err != nil {
			return "", err
		}
		if verb == "swarm-stop" {
			ps.rp.StopSwarm(s.swarm)
			return "", nil
		}
		if err := need(4); err != nil {
			return "", err
		}
		c, err := parseUint(args[2], 64)
		if err != nil {
			return "", err
		}
		pc, err := parseUint(args[3], 64)
		if err != nil {
			return "", err
		}
		return "", ps.rp.RunSwarm(s.swarm, codec.Channel(c), codec.Channel(pc))

	case "swarm-write":
		// swarm-write <name> <bytes> <bytes per second>: writes in the
		// background, then reports "written"
		if err := need(3); err != nil {
			return "", err
		}
		s, err := n.swarm(args[0])
		if err != nil {
			return "", err
		}
		if s.writer == nil {
			return "", errors.New("not a writer")
		}
		total, err := parseUint(args[1], 64)
		if err != nil {
			return "", err
		}
		rate, err := parseUint(args[2], 64)
		if err != nil {
			return "", err
		}
		go writeStream(args[0], s.writer, total, rate)
		return "", nil

	case "swarm-read":
		// swarm-read <name> <bytes>: reads in the background from the
		// reader's start to <bytes>, then reports "read-done"
		if err := need(2); err != nil {
			return "", err
		}
		s, err := n.swarm(args[0])
		if err != nil {
			return "", err
		}
		total, err := parseUint(args[1], 64)
		if err != nil {
			return "", err
		}
		go readStream(args[0], s.swarm, total)
		return "", nil

	case "exit":
		os.Exit(0)
	}
	return "", fmt.Errorf("unknown command %q", verb)
}

func writeStream(name string, w *ppspp.Writer, total, rate uint64) {
	const unit = 64 * 1024
	b := make([]byte, unit)
	start := time.Now()
	for off := uint64(0); off < total; off += unit {
		if rate != 0 {
			due := start.Add(time.Duration(float64(off) / float64(rate) * float64(time.Second)))
			time.Sleep(time.Until(due))
		}
		n := uint64(unit)
		if total-off < n {
			n = total - off
		}
		for i := range b[:n] {
			b[i] = streamByte(off + uint64(i))
		}
		if _, err := w.Write(b[:n]); err != nil {
			logf("write at %d: %v", off, err)
			return
		}
	}
	if err := w.Flush(); err != nil {
		logf("flush: %v", err)
	}
	emit("written %s %d", name, total)
}

func readStream(name string, s *ppspp.Swarm, total uint64) {
	r := s.Reader()
	start := r.Offset()
	h := sha256.New()
	var mismatches uint64
	off := start
	b := make([]byte, 64*1024)
	for off < total {
		n, err := r.Read(b)
		if err != nil {
			emit("read-error %s %d %s", name, off, hexs(err.Error()))
			return
		}
		if rest := total - off; uint64(n) > rest {
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
	emit("read-done %s %d %d %s %d", name, start, off-start, hex.EncodeToString(h.Sum(nil)), mismatches)
}

func main() {
	profileSeed := flag.String("profile-seed", "", "profile key seed (32 bytes hex)")
	hostSeed := flag.String("host-seed", "", "host key seed (32 bytes hex); random when empty")
	networkKey := flag.String("network", "", "network key (hex)")
	vpnPort := flag.Uint("vpn-port", 10000, "the vnic port every peer is bound to the network on")
	label := flag.String("label", "go", "host certificate subject")
	maxPeers := flag.Int("max-peers", 0, "vnic max peers (0: Go's default, which refuses peer exchange offers)")
	rateLimit := flag.Uint64("rate-limit", 0, "QoS upload cap in bytes per second (0: Go's default)")
	timeout := flag.Duration("timeout", 10*time.Minute, "exit after this long")
	verbose := flag.Bool("v", false, "log to stderr")
	listen := flag.Bool("listen", true, "serve the TCP and WebSocket interfaces (false: dial only)")
	webRTC := flag.Bool("webrtc", false, "add the WebRTC interface on a loopback UDP mux")
	flag.Parse()

	// The protocol owns the real stdout.
	out = os.Stdout
	os.Stdout = os.Stderr

	go func() {
		time.Sleep(*timeout)
		logf("timed out")
		os.Exit(3)
	}()

	if err := run(*profileSeed, *hostSeed, *networkKey, uint16(*vpnPort), *label, *maxPeers, *rateLimit, *verbose, *listen, *webRTC); err != nil {
		logf("error: %v", err)
		os.Exit(1)
	}
}

func run(profileSeed, hostSeed, networkKey string, vpnPort uint16, label string, maxPeers int, rateLimit uint64, verbose, listen, webRTC bool) error {
	logger := zap.NewNop()
	if verbose {
		logger, _ = zap.NewDevelopment()
	}

	seed, err := hex.DecodeString(profileSeed)
	if err != nil || len(seed) != ed25519.SeedSize {
		return errors.New("-profile-seed: want 32 bytes of hex")
	}
	profileKey := keyFromSeed(seed)
	nk, err := hex.DecodeString(networkKey)
	if err != nil || len(nk) == 0 {
		return errors.New("-network: want hex")
	}

	// cmd/svc: a TCP mux, and the WebSocket interface on the HTTP mux.
	// Without -listen, the interfaces dial but advertise nothing.
	tcpOpts := vnic.TCPInterfaceOptions{}
	wsOpts := vnic.WSInterfaceOptions{AllowInsecure: true}
	if listen {
		mux, tcpLis, err := vnic.NewTCPMux(logger, "127.0.0.1:0")
		if err != nil {
			return err
		}
		httpMux := httputil.NewMapServeMux()
		httpLis, err := net.Listen("tcp", "127.0.0.1:0")
		if err != nil {
			return err
		}
		go http.Serve(httpLis, httpMux)
		tcpOpts.Address = tcpLis.Addr().String()
		tcpOpts.Mux = mux
		wsOpts.ServeMux = httpMux
		wsOpts.Address = httpLis.Addr().String()
	}
	tcp := vnic.NewTCPInterface(logger, tcpOpts)
	ws := vnic.NewWSInterface(logger, wsOpts)
	opts := []vnic.HostOption{vnic.WithLabel(label), vnic.WithInterface(tcp), vnic.WithInterface(ws)}

	// cmd/svc's UDPMuxAddress, with no ICE servers (host candidates on
	// loopback only).
	udpAddr := "-"
	if webRTC {
		udpMux, udpLis, err := vnic.NewWebRTCUDPMux("127.0.0.1:0")
		if err != nil {
			return err
		}
		udpAddr = udpLis.LocalAddr().String()
		wrtc := vnic.NewWebRTCInterface(logger, &vnic.WebRTCInterfaceOptions{
			ICEServers: []string{},
			UDPMux:     udpMux,
		})
		opts = append(opts, vnic.WithInterface(wrtc))
	}
	if rateLimit != 0 {
		opts = append(opts, vnic.WithRateLimit(rateLimit))
	}
	h, err := vnic.New(logger, profileKey, opts...)
	if err != nil {
		return err
	}
	if hostSeed != "" {
		hs, err := hex.DecodeString(hostSeed)
		if err != nil || len(hs) != ed25519.SeedSize {
			return errors.New("-host-seed: want 32 bytes of hex")
		}
		setHostKey(h, keyFromSeed(hs))
	}
	if maxPeers != 0 {
		h.SetMaxPeers(maxPeers)
	}
	kick(h.QOS().Class)

	v, err := vpn.New(logger, h)
	if err != nil {
		return err
	}
	vn, err := v.AddNetwork(nk)
	if err != nil {
		return err
	}

	n := &node{
		host:        h,
		vpn:         vn,
		vpnPort:     vpnPort,
		runner:      ppspp.NewRunner(context.Background(), logger),
		transferQOS: h.QOS().AddClass(1),
		peers:       map[kademlia.ID]*peerState{},
		swarms:      map[string]*swarmState{},
	}
	h.AddPeerHandler(vnic.PeerHandlerFunc(n.handlePeer))

	// vnic.New starts the interfaces' Listen in goroutines; wait until
	// both advertise their URIs. Only the TCP and WebSocket candidates are
	// asked: a WebRTC candidate would open a PeerConnection.
	tcpURI, wsURI := "-", "-"
	for i := 0; listen; i++ {
		tcpURI, wsURI = "", ""
		for _, iface := range []vnic.Interface{tcp, ws} {
			c, err := iface.CreateLinkCandidate(context.Background(), h)
			if err != nil {
				continue
			}
			if d, err := c.LocalDescription(); err == nil && d != nil {
				switch d.Interface {
				case "tcp":
					tcpURI = d.Description
				case "ws":
					wsURI = d.Description
				}
			}
		}
		if tcpURI != "" && wsURI != "" {
			break
		}
		if i > 500 {
			return errors.New("interfaces never listened")
		}
		time.Sleep(10 * time.Millisecond)
	}
	emit("ready %s %s %s %s", idHex(h.ID()), tcpURI, wsURI, udpAddr)

	s := bufio.NewScanner(os.Stdin)
	s.Buffer(make([]byte, 1<<20), 1<<20)
	for s.Scan() {
		f := strings.Fields(s.Text())
		if len(f) < 2 {
			continue
		}
		id, verb, args := f[0], f[1], f[2:]
		go func() {
			v, err := n.command(verb, args)
			if err != nil {
				emit("err %s %s", id, hexs(err.Error()))
				return
			}
			if v == "" {
				v = "-"
			}
			emit("ok %s %s", id, v)
		}()
	}
	// stdin closed: the test is gone.
	return nil
}
