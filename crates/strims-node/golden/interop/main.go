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
//     channel ids through RunnerPeer.RunSwarm;
//   - with -peer-rpc, internal/peer's RPC wiring: per peer a ChannelPair on
//     PeerRPCClientPort and PeerRPCServerPort in its own QoS class, an
//     rpc.Client on the first and an rpc.Server on the second, serving the
//     test Debug service in crates/strims-rpc/golden/debugsvc. The rpc-*
//     verbs call the peer's.
//   - with -peer-rpc, profile replication on demand (the repl-* verbs; see
//     replication.go);
//   - with -transfer and -peer-rpc, internal/transfer in place of the
//     ppspp wiring above: swarms bind by announce (the transfer-* verbs;
//     see transfer.go);
//   - with -network-control and -peer-rpc, internal/network's control in
//     place of the fixed port pair: peers bind to the networks of a
//     profile store by negotiation (the net-* verbs; see network.go), and
//     the control's internal/transfer runs every peer's swarms in place of
//     the ppspp wiring above;
//   - with -directory and -network-control, internal/directory's control
//     on the same profile store (the dir-* verbs; see directory.go);
//   - with -chat and -directory, internal/chat's control on the same
//     profile store (the chat-* verbs; see chat.go);
//   - with -network-control, a video swarm written as internal/videoingress
//     writes a channel's, and streams read through internal/videoegress's
//     control (the video-* verbs; see video.go);
//   - with -network-seed, internal/network/dialer's RPC over vpn: a host
//     certificate chained to the network key's root, as
//     Dialer.ReplaceOrInsertNetwork issues it. vpn-serve serves the Debug
//     service on a vpn port (a VPNServerDialer, optionally publishing the
//     address in the hash table), and vpn-rpc and vpn-rpc-dht make an rpc-*
//     call through a VPNDialer to a fixed or a published address.
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
// An rpc-* verb whose call fails with an RPC error answers "ok <id> <code>
// <message hex>".
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

	"github.com/MemeLabs/protobuf/pkg/rpc"
	"github.com/MemeLabs/strims/crates/strims-rpc/golden/debugsvc"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/network/dialer"
	debugv1 "github.com/MemeLabs/strims/pkg/apis/debug/v1"
	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"github.com/MemeLabs/strims/pkg/httputil"
	"github.com/MemeLabs/strims/pkg/kademlia"
	"github.com/MemeLabs/strims/pkg/kv/kvtest"
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
	// With -peer-rpc: the peer's Debug service, and the raw client under it.
	debug  *debugv1.DebugClient
	caller *rpc.Client
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
	// With -peer-rpc: the RPC channels' QoS class. With -peer-rpc or
	// -network-seed: the served service.
	rpcQOS *qos.Class
	debug  *debugsvc.Service
	// With -network-seed: this host's certificate on the network.
	vpnCert *certificate.Certificate
	logger  *zap.Logger
	// Profile replication (repl-* verbs): the vpn host it runs on, the
	// node's profile key and network key, the kv its stores live in, and
	// the running replication once set up.
	vpnHost    *vpn.Host
	profileKey *key.Key
	networkKey []byte
	replBlob   *kvtest.Store
	repl       *replState
	// With -transfer: internal/transfer runs every peer's swarms.
	transfer *transferState
	// With -network-control: internal/network binds peers to networks.
	network *networkState
	// The video-* verbs' writer and egress control.
	video videoState

	mu     sync.Mutex
	peers  map[kademlia.ID]*peerState
	swarms map[string]*swarmState
}

func (n *node) handlePeer(p *vnic.Peer) {
	id := p.HostID()
	ps := &peerState{peer: p}
	var w *vnic.FrameWriter
	if n.transfer == nil && n.network == nil {
		w = vnic.NewFrameWriter(p.Link, vnic.TransferPort, n.transferQOS)
		cr, rp := n.runner.RunPeer(id.Bytes(nil), w)
		p.SetHandler(vnic.TransferPort, func(_ *vnic.Peer, f vnic.Frame) error {
			err := cr.HandleMessage(f.Body)
			if err != nil {
				emit("transfer-error %s %s", idHex(id), hexs(err.Error()))
			}
			return err
		})
		ps.rp = rp
	}

	var transferPeer uint64
	if n.rpcQOS != nil {
		rw0, rw1 := p.ChannelPair(vnic.PeerRPCClientPort, vnic.PeerRPCServerPort, n.rpcQOS)
		c, err := rpc.NewClient(n.logger, &rpc.RWFDialer{Logger: n.logger, ReadWriteFlusher: rw0})
		if err != nil {
			logf("rpc client: %v", err)
		} else {
			ps.caller = c
			ps.debug = debugv1.NewDebugClient(c)
		}
		s := rpc.NewServer(n.logger, &rpc.RWFDialer{Logger: n.logger, ReadWriteFlusher: rw1})
		debugv1.RegisterDebugService(s, n.debug)
		if ps.caller != nil {
			n.replAddPeer(id, p, s, ps.caller)
			if n.transfer != nil {
				transferPeer = n.transferAddPeer(p, s, ps.caller)
			}
			if n.network != nil {
				n.networkAddPeer(id, p, s, ps.caller)
			}
		}
		go s.Listen(context.Background())
	}
	n.mu.Lock()
	n.peers[id] = ps
	n.mu.Unlock()

	// The host may still be bound through a replaced peer's link.
	if n.network == nil {
		n.vpn.Network.RemovePeer(id)
		n.vpn.Network.AddPeer(p, n.vpnPort, n.vpnPort)
	}
	if transferPeer != 0 {
		n.transferNetworkPeerOpen(transferPeer)
	}
	emit("peer %s %s %s %s", idHex(id), hexs(p.NodePlatform), hexs(p.NodeVersion), linkType(p.Link))

	go func() {
		<-p.Done()
		if ps.rp != nil {
			ps.rp.Stop()
			w.Close()
		}
		if transferPeer != 0 {
			n.transferRemovePeer(transferPeer)
		}
		if ps.caller != nil {
			ps.caller.Close()
		}
		n.mu.Lock()
		current := n.peers[id] == ps
		if current {
			delete(n.peers, id)
		}
		n.mu.Unlock()
		if current && n.network == nil {
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
		if ps.rp == nil {
			return "", errors.New("transfers run the swarms")
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

	case "rpc-pprof", "rpc-watch", "rpc-watch-cancel", "rpc-read-deadline", "rpc-cancels",
		"rpc-config", "rpc-negative", "rpc-raw":
		// rpc-<verb> <peer> [<args>]
		if err := need(1); err != nil {
			return "", err
		}
		ps, err := n.peer(args[0])
		if err != nil {
			return "", err
		}
		if ps.debug == nil {
			return "", errors.New("no peer rpc")
		}
		return rpcCommand(ctx, verb, ps, args[1:])

	case "vpn-serve":
		// vpn-serve <port> [<key seed> <salt>]: serves the Debug service on
		// the vpn port, publishing its address under the key and salt
		if err := need(1); err != nil {
			return "", err
		}
		if n.vpnCert == nil {
			return "", errors.New("no -network-seed")
		}
		port, err := parseUint(args[0], 16)
		if err != nil {
			return "", err
		}
		d := &dialer.VPNServerDialer{Logger: n.logger, Node: n.vpn, Port: uint16(port), CertFunc: n.certificate}
		if len(args) >= 3 {
			seed, err := unhex(args[1])
			if err != nil || len(seed) != ed25519.SeedSize {
				return "", errors.New("bad key seed")
			}
			salt, err := unhex(args[2])
			if err != nil {
				return "", err
			}
			d.Publisher = &dialer.DHTHostAddrPublisher{Key: keyFromSeed(seed), Salt: salt}
		}
		s := rpc.NewServer(n.logger, d)
		debugv1.RegisterDebugService(s, n.debug)
		go func() {
			if err := s.Listen(context.Background()); err != nil {
				logf("vpn server: %v", err)
			}
		}()
		for i := 0; network.Handler(uint16(port)) == nil; i++ {
			if i > 500 {
				return "", errors.New("vpn server never listened")
			}
			time.Sleep(10 * time.Millisecond)
		}
		return "", nil

	case "vpn-rpc", "vpn-rpc-dht":
		// vpn-rpc <host> <port> <verb> [<args>],
		// vpn-rpc-dht <key> <salt> <verb> [<args>]: the rpc-<verb> call
		// (see rpcCommand) to the Debug service at the address, over vpn
		if err := need(3); err != nil {
			return "", err
		}
		if n.vpnCert == nil {
			return "", errors.New("no -network-seed")
		}
		var resolver dialer.HostAddrResolver
		if verb == "vpn-rpc" {
			id, err := parseID(args[0])
			if err != nil {
				return "", err
			}
			port, err := parseUint(args[1], 16)
			if err != nil {
				return "", err
			}
			resolver = &dialer.StaticHostAddrResolver{HostAddr: dialer.HostAddr{HostID: id, Port: uint16(port)}}
		} else {
			k, err := unhex(args[0])
			if err != nil {
				return "", err
			}
			salt, err := unhex(args[1])
			if err != nil {
				return "", err
			}
			resolver = &dialer.DHTHostAddrResolver{Key: k, Salt: salt}
		}
		c, err := rpc.NewClient(n.logger, &dialer.VPNDialer{Logger: n.logger, Node: n.vpn, Resolver: resolver, CertFunc: n.certificate})
		if err != nil {
			return "", err
		}
		defer c.Close()
		tctx, cancel := context.WithTimeout(ctx, 20*time.Second)
		defer cancel()
		ps := &peerState{debug: debugv1.NewDebugClient(c), caller: c}
		return rpcCommand(tctx, "rpc-"+args[2], ps, args[3:])

	case "exit":
		os.Exit(0)
	}
	if strings.HasPrefix(verb, "repl-") {
		return n.replCommand(verb, args)
	}
	if strings.HasPrefix(verb, "transfer-") {
		return n.transferCommand(verb, args)
	}
	if strings.HasPrefix(verb, "net-") {
		return n.networkCommand(verb, args)
	}
	if strings.HasPrefix(verb, "dir-") {
		return n.directoryCommand(verb, args)
	}
	if strings.HasPrefix(verb, "chat-") {
		return n.chatCommand(verb, args)
	}
	if strings.HasPrefix(verb, "video-") {
		return n.videoCommand(verb, args)
	}
	return "", fmt.Errorf("unknown command %q", verb)
}

func (n *node) certificate() *certificate.Certificate {
	return n.vpnCert
}

// hostCertificate issues the profile's membership certificate with the
// network key from networkSeed, under a self-signed root (as
// dao.NewNetwork does), and the host's certificate under it (as
// Dialer.ReplaceOrInsertNetwork does).
func hostCertificate(networkSeed []byte, profileKey, hostKey *key.Key, label string) (*certificate.Certificate, error) {
	const ttl = 24 * time.Hour
	networkKey := keyFromSeed(networkSeed)
	root, err := dao.NewSelfSignedCertificate(networkKey, certificate.KeyUsage_KEY_USAGE_SIGN, ttl, dao.WithSubject("network"))
	if err != nil {
		return nil, err
	}
	csr, err := dao.NewCertificateRequest(profileKey, certificate.KeyUsage_KEY_USAGE_PEER|certificate.KeyUsage_KEY_USAGE_SIGN, dao.WithSubject(label))
	if err != nil {
		return nil, err
	}
	member, err := dao.SignCertificateRequest(csr, ttl, networkKey)
	if err != nil {
		return nil, err
	}
	member.ParentOneof = &certificate.Certificate_Parent{Parent: root}
	csr, err = dao.NewCertificateRequest(hostKey, certificate.KeyUsage_KEY_USAGE_SIGN)
	if err != nil {
		return nil, err
	}
	cert, err := dao.SignCertificateRequest(csr, ttl, profileKey)
	if err != nil {
		return nil, err
	}
	cert.ParentOneof = &certificate.Certificate_Parent{Parent: member}
	return cert, nil
}

// rpcError formats an RPC error as "<code> <message hex>".
func rpcError(err error) (string, error) {
	var e interface{ ErrorCode() int32 }
	if !errors.As(err, &e) {
		return "", err
	}
	return fmt.Sprintf("%d %s", rpc.ErrorCode(err), hexs(err.Error())), nil
}

// watch runs a WatchMetrics call, handing items to f until it returns
// false, and returns the call's error.
func watch(ctx context.Context, c *debugv1.DebugClient, interval int32, f func(*debugv1.WatchMetricsResponse) bool) error {
	ctx, cancel := context.WithCancel(ctx)
	defer cancel()
	ch := make(chan *debugv1.WatchMetricsResponse)
	done := make(chan error, 1)
	go func() {
		done <- c.WatchMetrics(ctx, &debugv1.WatchMetricsRequest{IntervalMs: interval}, ch)
	}()
	for item := range ch {
		if !f(item) {
			cancel()
			for range ch {
			}
			break
		}
	}
	return <-done
}

// rpcCommand calls the peer's Debug service:
//
//	rpc-pprof <peer> <size> <name length>: "<name> <data length> <ok|bad>"
//	  (the name spells size, or is name length bytes long when nonzero)
//	rpc-watch <peer> <n>: "<items> <data hex>"
//	rpc-watch-cancel <peer> <k>: cancels an endless stream after k items
//	rpc-read-deadline <peer> <ms>: a ReadMetrics call that times out
//	rpc-cancels <peer>: the peer service's count of cancelled calls
//	rpc-config <peer>, rpc-negative <peer>: the RPC error
//	rpc-raw <peer> <method>: a call of any method; the RPC error
func rpcCommand(ctx context.Context, verb string, ps *peerState, args []string) (string, error) {
	c := ps.debug
	arg := func(i int) (int, error) {
		if len(args) <= i {
			return 0, fmt.Errorf("%s: missing argument %d", verb, i+1)
		}
		return strconv.Atoi(args[i])
	}
	switch verb {
	case "rpc-pprof":
		size, err := arg(0)
		if err != nil {
			return "", err
		}
		nameLen, err := arg(1)
		if err != nil {
			return "", err
		}
		name := strconv.Itoa(size)
		if nameLen > 0 {
			name = strings.Repeat("y", nameLen)
		}
		res := &debugv1.PProfResponse{}
		if err := c.PProf(ctx, &debugv1.PProfRequest{Name: name}, res); err != nil {
			return "", err
		}
		ok := "ok"
		if string(res.Data) != string(debugsvc.Data(size)) {
			ok = "bad"
		}
		return fmt.Sprintf("%s %d %s", res.Name, len(res.Data), ok), nil

	case "rpc-watch":
		k, err := arg(0)
		if err != nil {
			return "", err
		}
		var got []byte
		err = watch(ctx, c, int32(k), func(r *debugv1.WatchMetricsResponse) bool {
			got = append(got, r.Data...)
			return true
		})
		if err != nil {
			return "", err
		}
		return fmt.Sprintf("%d %s", len(got), hexs(string(got))), nil

	case "rpc-watch-cancel":
		k, err := arg(0)
		if err != nil {
			return "", err
		}
		seen := 0
		err = watch(ctx, c, 0, func(*debugv1.WatchMetricsResponse) bool {
			seen++
			return seen < k
		})
		if !errors.Is(err, context.Canceled) {
			return "", fmt.Errorf("call ended with %v", err)
		}
		return "canceled", nil

	case "rpc-read-deadline":
		ms, err := arg(0)
		if err != nil {
			return "", err
		}
		tctx, cancel := context.WithTimeout(ctx, time.Duration(ms)*time.Millisecond)
		defer cancel()
		err = c.ReadMetrics(tctx, &debugv1.ReadMetricsRequest{}, &debugv1.ReadMetricsResponse{})
		if !errors.Is(err, context.DeadlineExceeded) {
			return "", fmt.Errorf("call ended with %v", err)
		}
		return "deadline", nil

	case "rpc-cancels":
		res := &debugv1.StartMockStreamResponse{}
		if err := c.StartMockStream(ctx, &debugv1.StartMockStreamRequest{}, res); err != nil {
			return "", err
		}
		return strconv.FormatUint(res.Id, 10), nil

	case "rpc-config":
		return rpcError(c.GetConfig(ctx, &debugv1.GetConfigRequest{}, &debugv1.GetConfigResponse{}))

	case "rpc-negative":
		return rpcError(watch(ctx, c, -1, func(*debugv1.WatchMetricsResponse) bool { return true }))

	case "rpc-raw":
		if len(args) < 1 {
			return "", errors.New("rpc-raw: missing method")
		}
		return rpcError(ps.caller.CallUnary(ctx, args[0], &debugv1.PProfRequest{}, &debugv1.PProfResponse{}))
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
	peerRPC := flag.Bool("peer-rpc", false, "run RPC on every peer's channel pair (ports 4 and 5)")
	networkSeed := flag.String("network-seed", "", "network key seed (32 bytes hex): issue a host certificate for RPC over vpn")
	transfers := flag.Bool("transfer", false, "run swarms through internal/transfer (needs -peer-rpc)")
	networkControl := flag.Bool("network-control", false, "bind peers to networks through internal/network (needs -peer-rpc)")
	directory := flag.Bool("directory", false, "run internal/directory on the profile store (needs -network-control)")
	chat := flag.Bool("chat", false, "run internal/chat on the profile store (needs -directory)")
	flag.Parse()

	// The protocol owns the real stdout.
	out = os.Stdout
	os.Stdout = os.Stderr

	go func() {
		time.Sleep(*timeout)
		logf("timed out")
		os.Exit(3)
	}()

	if err := run(*profileSeed, *hostSeed, *networkKey, *networkSeed, uint16(*vpnPort), *label, *maxPeers, *rateLimit, *verbose, *listen, *webRTC, *peerRPC, *transfers, *networkControl, *directory, *chat); err != nil {
		logf("error: %v", err)
		os.Exit(1)
	}
}

func run(profileSeed, hostSeed, networkKey, networkSeed string, vpnPort uint16, label string, maxPeers int, rateLimit uint64, verbose, listen, webRTC, peerRPC, transfers, networkControl, directory, chat bool) error {
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
		logger:      logger,
		vpnHost:     v,
		profileKey:  profileKey,
		networkKey:  nk,
		replBlob:    newReplBlob(),
	}
	if peerRPC {
		n.rpcQOS = h.QOS().AddClass(1)
	}
	if transfers {
		if !peerRPC {
			return errors.New("-transfer needs -peer-rpc")
		}
		if n.transfer, err = newTransferState(logger, v); err != nil {
			return err
		}
	}
	if directory && !networkControl {
		return errors.New("-directory needs -network-control")
	}
	if chat && !directory {
		return errors.New("-chat needs -directory")
	}
	if networkControl {
		if !peerRPC || transfers {
			return errors.New("-network-control needs -peer-rpc, without -transfer")
		}
		if n.network, err = newNetworkState(logger, v, profileKey, label, directory, chat); err != nil {
			return err
		}
	}
	if peerRPC || networkSeed != "" {
		n.debug = &debugsvc.Service{}
	}
	if networkSeed != "" {
		ns, err := hex.DecodeString(networkSeed)
		if err != nil || len(ns) != ed25519.SeedSize {
			return errors.New("-network-seed: want 32 bytes of hex")
		}
		if n.vpnCert, err = hostCertificate(ns, profileKey, h.Key(), label); err != nil {
			return err
		}
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
