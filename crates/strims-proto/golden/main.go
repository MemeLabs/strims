// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

// Generates golden vectors for crates/strims-proto.
//
// Run from the repo root:
//
//	go run -tags golden ./crates/strims-proto/golden
//
// Each line of vectors.hex is "<name> <protobuf hex> <stable hex>" ("=" for
// empty), where the stable encoding is internal/dao's signableMessageEncoder
// output ("-" for messages it can't encode, and for protobuf-only vectors).
// Maps have at most one entry: Go's map order isn't canonical.
package main

import (
	"bufio"
	"encoding/hex"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"unsafe"

	rpcapi "github.com/MemeLabs/protobuf/pkg/apis/rpc"
	chatv1 "github.com/MemeLabs/strims/pkg/apis/chat/v1"
	debugv1 "github.com/MemeLabs/strims/pkg/apis/debug/v1"
	ppsppv1 "github.com/MemeLabs/strims/pkg/apis/devtools/v1/ppspp"
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
	"github.com/MemeLabs/strims/pkg/apis/type/image"
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"github.com/MemeLabs/strims/pkg/apis/type/swarm"
	vnicv1 "github.com/MemeLabs/strims/pkg/apis/vnic/v1"
	vpnv1 "github.com/MemeLabs/strims/pkg/apis/vpn/v1"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/types/known/wrapperspb"

	// linked below
	_ "github.com/MemeLabs/strims/internal/dao"
)

// The encoder is unexported; link its methods directly so the vectors come
// from the real implementation. The receiver is a pointer to an empty struct.
//
//go:linkname stableLen github.com/MemeLabs/strims/internal/dao.(*signableMessageEncoder).len
func stableLen(e unsafe.Pointer, rv reflect.Value) int

//go:linkname stableEncode github.com/MemeLabs/strims/internal/dao.(*signableMessageEncoder).encode
func stableEncode(e unsafe.Pointer, rv reflect.Value, b []byte) int

func stable(m proto.Message) (b []byte, ok bool) {
	defer func() {
		if recover() != nil {
			b, ok = nil, false
		}
	}()
	var e struct{}
	rv := reflect.ValueOf(m)
	b = make([]byte, stableLen(unsafe.Pointer(&e), rv))
	n := stableEncode(unsafe.Pointer(&e), rv, b)
	if n != len(b) {
		panic("length mismatch")
	}
	return b, true
}

func seq(start byte, n int) []byte {
	b := make([]byte, n)
	for i := range b {
		b[i] = start + byte(i)
	}
	return b
}

func rootCert() *certificate.Certificate {
	return &certificate.Certificate{
		Key:          seq(0x10, 32),
		KeyType:      key.KeyType_KEY_TYPE_ED25519,
		KeyUsage:     certificate.KeyUsage_KEY_USAGE_SIGN,
		Subject:      "root",
		NotBefore:    1_600_000_000,
		NotAfter:     1_900_000_000,
		SerialNumber: seq(0x30, 16),
		Signature:    seq(0x40, 64),
	}
}

func certChain() *certificate.Certificate {
	ca := &certificate.Certificate{
		Key:          seq(0x50, 32),
		KeyType:      key.KeyType_KEY_TYPE_ED25519,
		KeyUsage:     certificate.KeyUsage_KEY_USAGE_SIGN | certificate.KeyUsage_KEY_USAGE_PEER,
		Subject:      "intermediate",
		NotBefore:    1_600_000_100,
		NotAfter:     1_800_000_000,
		SerialNumber: seq(0x70, 16),
		Signature:    seq(0x80, 64),
		ParentOneof:  &certificate.Certificate_Parent{Parent: rootCert()},
	}
	return &certificate.Certificate{
		Key:          seq(0xa0, 32),
		KeyType:      key.KeyType_KEY_TYPE_ED25519,
		KeyUsage:     certificate.KeyUsage_KEY_USAGE_PEER | certificate.KeyUsage_KEY_USAGE_BOOTSTRAP,
		Subject:      "leaf ✓",
		NotBefore:    1_700_000_000,
		NotAfter:     1_700_086_400,
		SerialNumber: seq(0xc0, 16),
		Signature:    seq(0x01, 64),
		ParentOneof:  &certificate.Certificate_Parent{Parent: ca},
	}
}

func hashTableRecord() *vpnv1.HashTableMessage_Record {
	return &vpnv1.HashTableMessage_Record{
		Key:       seq(0x11, 32),
		Salt:      []byte("salt"),
		Value:     seq(0x00, 200),
		Timestamp: 1_700_000_000_123,
		Signature: seq(0x22, 64),
	}
}

func peerIndexRecord(i byte) *vpnv1.PeerIndexMessage_Record {
	return &vpnv1.PeerIndexMessage_Record{
		Hash:      seq(i, 32),
		HostId:    seq(i+1, 32),
		Port:      uint32(1000 + int(i)),
		Timestamp: -int64(i) * 1000,
		Key:       seq(i+2, 32),
		Signature: seq(i+3, 64),
	}
}

func styleSheet() *chatv1.StyleSheet {
	return &chatv1.StyleSheet{
		Scss: ".a { b: c }",
		Css:  ".a{b:c}",
		Assets: []*chatv1.StyleSheet_Asset{
			{Name: "one", Image: &image.Image{Type: image.ImageType_IMAGE_TYPE_PNG, Height: 28, Width: 32, Data: seq(0x89, 8)}},
			{Name: "two"},
		},
	}
}

type vector struct {
	name string
	msg  proto.Message
}

func marshal(m proto.Message) []byte {
	b, err := proto.Marshal(m)
	if err != nil {
		panic(err)
	}
	return b
}

// rpcCall builds a Call the way the RPC library's send does: the argument
// marshaled on its own, and an empty header map.
func rpcCall(id, parentID uint64, method string, kind rpcapi.Call_Kind, arg proto.Message) *rpcapi.Call {
	return &rpcapi.Call{
		Id:       id,
		ParentId: parentID,
		Method:   method,
		Kind:     kind,
		Argument: marshal(arg),
		Headers:  map[string][]byte{},
	}
}

// rpcVectors are protobuf-only: the envelope has no stable encoding.
func rpcVectors() []vector {
	header := rpcCall(9, 0, "strims.debug.v1.Debug.ReadMetrics", rpcapi.Call_CALL_KIND_DEFAULT, &debugv1.ReadMetricsRequest{Format: debugv1.MetricsFormat_METRICS_FORMAT_PROTO_DELIM})
	header.Headers["certificate"] = marshal(certChain())
	return []vector{
		{"rpc_call_request", rpcCall(1, 0, "strims.debug.v1.Debug.PProf", rpcapi.Call_CALL_KIND_DEFAULT, &debugv1.PProfRequest{Name: "heap", Debug: true})},
		{"rpc_call_callback", rpcCall(2, 1, "CALLBACK", rpcapi.Call_CALL_KIND_DEFAULT, &debugv1.PProfResponse{Name: "heap", Data: seq(0x00, 40)})},
		{"rpc_call_error", rpcCall(3, 1, "CALLBACK", rpcapi.Call_CALL_KIND_ERROR, &rpcapi.Error{Message: "boom", Code: 5})},
		{"rpc_call_error_negative_code", rpcCall(4, 1, "CALLBACK", rpcapi.Call_CALL_KIND_ERROR, &rpcapi.Error{Message: "negative", Code: -3})},
		{"rpc_call_close", rpcCall(5, 1, "CALLBACK", rpcapi.Call_CALL_KIND_CLOSE, &rpcapi.Close{})},
		{"rpc_call_undefined", rpcCall(6, 1, "CALLBACK", rpcapi.Call_CALL_KIND_UNDEFINED, &rpcapi.Undefined{})},
		{"rpc_call_cancel", rpcCall(7, 1, "CANCEL", rpcapi.Call_CALL_KIND_CANCEL, &rpcapi.Cancel{})},
		{"rpc_call_cancel_default_kind", rpcCall(8, 1, "CANCEL", rpcapi.Call_CALL_KIND_DEFAULT, &rpcapi.Cancel{})},
		{"rpc_call_header", header},
		{"rpc_call_max_ids", rpcCall(^uint64(0), ^uint64(0)-1, "strims.debug.v1.Debug.WatchMetrics", rpcapi.Call_CALL_KIND_DEFAULT, &debugv1.WatchMetricsRequest{IntervalMs: -1})},
		{"rpc_call_empty", &rpcapi.Call{}},
		{"rpc_error", &rpcapi.Error{Message: "method not found: strims.debug.v1.Debug.Nope", Code: 7}},
	}
}

func vectors() []vector {
	return []vector{
		{"type_certificate_chain", certChain()},
		{"type_certificate_parent_serial", &certificate.Certificate{
			Key:          seq(0x01, 32),
			KeyType:      key.KeyType_KEY_TYPE_ED25519,
			KeyUsage:     certificate.KeyUsage_KEY_USAGE_PEER,
			NotBefore:    1,
			NotAfter:     ^uint64(0),
			SerialNumber: seq(0x02, 16),
			Signature:    seq(0x03, 64),
			ParentOneof:  &certificate.Certificate_ParentSerialNumber{ParentSerialNumber: seq(0x30, 16)},
		}},
		{"type_certificate_request", &certificate.CertificateRequest{
			Key:       seq(0x10, 32),
			KeyType:   key.KeyType_KEY_TYPE_ED25519,
			KeyUsage:  certificate.KeyUsage_KEY_USAGE_PEER | certificate.KeyUsage_KEY_USAGE_ENCIPHERMENT,
			Subject:   "csr",
			Signature: seq(0x20, 64),
		}},
		{"type_key", &key.Key{Type: key.KeyType_KEY_TYPE_ED25519, Private: seq(0x00, 64), Public: seq(0x20, 32)}},
		{"type_image_negative_enum", &image.Image{Type: image.ImageType(-1), Height: 1, Width: 2}},
		{"vnic_peer_init", &vnicv1.PeerInit{
			ProtocolVersion: 1,
			Certificate:     certChain(),
			NodePlatform:    "linux",
			NodeVersion:     "v1.2.3",
		}},
		{"vnic_peer_init_empty_certificate", &vnicv1.PeerInit{ProtocolVersion: 1, Certificate: &certificate.Certificate{}}},
		{"vnic_aes_link_init", &vnicv1.AESLinkInit{ProtocolVersion: 1, Key: seq(0x40, 32), Iv: seq(0x60, 16)}},
		{"vpn_hash_table_record", hashTableRecord()},
		{"vpn_hash_table_publish", &vpnv1.HashTableMessage{Body: &vpnv1.HashTableMessage_Publish_{
			Publish: &vpnv1.HashTableMessage_Publish{Record: hashTableRecord()},
		}}},
		{"vpn_hash_table_unpublish", &vpnv1.HashTableMessage{Body: &vpnv1.HashTableMessage_Unpublish_{
			Unpublish: &vpnv1.HashTableMessage_Unpublish{Record: hashTableRecord()},
		}}},
		{"vpn_hash_table_get_request", &vpnv1.HashTableMessage{Body: &vpnv1.HashTableMessage_GetRequest_{
			GetRequest: &vpnv1.HashTableMessage_GetRequest{RequestId: ^uint64(0) - 1, Hash: seq(0x33, 32), IfModifiedSince: -5},
		}}},
		{"vpn_hash_table_get_response", &vpnv1.HashTableMessage{Body: &vpnv1.HashTableMessage_GetResponse_{
			GetResponse: &vpnv1.HashTableMessage_GetResponse{RequestId: 7, Record: hashTableRecord()},
		}}},
		{"vpn_hash_table_publish_empty_record", &vpnv1.HashTableMessage{Body: &vpnv1.HashTableMessage_Publish_{
			Publish: &vpnv1.HashTableMessage_Publish{Record: &vpnv1.HashTableMessage_Record{}},
		}}},
		{"vpn_peer_index_record", peerIndexRecord(5)},
		{"vpn_peer_index_publish", &vpnv1.PeerIndexMessage{Body: &vpnv1.PeerIndexMessage_Publish_{
			Publish: &vpnv1.PeerIndexMessage_Publish{Record: peerIndexRecord(1)},
		}}},
		{"vpn_peer_index_search_request", &vpnv1.PeerIndexMessage{Body: &vpnv1.PeerIndexMessage_SearchRequest_{
			SearchRequest: &vpnv1.PeerIndexMessage_SearchRequest{RequestId: 99, Hash: seq(0x44, 32)},
		}}},
		{"vpn_peer_index_search_response", &vpnv1.PeerIndexMessage{Body: &vpnv1.PeerIndexMessage_SearchResponse_{
			SearchResponse: &vpnv1.PeerIndexMessage_SearchResponse{
				RequestId: 99,
				Records:   []*vpnv1.PeerIndexMessage_Record{peerIndexRecord(1), peerIndexRecord(100), peerIndexRecord(200)},
			},
		}}},
		{"vpn_peer_exchange_link_offer", &vpnv1.PeerExchangeMessage{Body: &vpnv1.PeerExchangeMessage_LinkOffer_{
			LinkOffer: &vpnv1.PeerExchangeMessage_LinkOffer{
				ExchangeId: 1 << 40,
				Descriptions: []*vnicv1.LinkDescription{
					{Interface: "webrtc", Description: `{"type":"offer","sdp":"v=0"}`},
					{Interface: "tcp", Description: "tcp://127.0.0.1:8083/abcd"},
				},
			},
		}}},
		{"vpn_peer_exchange_link_answer", &vpnv1.PeerExchangeMessage{Body: &vpnv1.PeerExchangeMessage_LinkAnswer_{
			LinkAnswer: &vpnv1.PeerExchangeMessage_LinkAnswer{
				ExchangeId:   2,
				Descriptions: []*vnicv1.LinkDescription{{Interface: "webrtc", Description: "answer"}},
				ErrorMessage: "partial failure",
			},
		}}},
		{"vpn_peer_exchange_rejection", &vpnv1.PeerExchangeMessage{Body: &vpnv1.PeerExchangeMessage_Rejection_{
			Rejection: &vpnv1.PeerExchangeMessage_Rejection{MediationId: 3},
		}}},
		{"type_swarm_cache", &swarm.Cache{
			Id:  42,
			Uri: "magnet:?xt=urn:ppspp:abcd",
			Integrity: &swarm.Cache_Integrity{
				SignAllIntegrity: &swarm.Cache_SignAllIntegrity{Timestamps: []int64{1, -2, 1 << 40}, Signatures: seq(0x00, 128)},
				MerkleIntegrity:  &swarm.Cache_MerkleIntegrity{Timestamps: []int64{3}, Signatures: [][]byte{seq(0x01, 64), {}, seq(0x02, 64)}},
			},
			Data:  seq(0x10, 300),
			Epoch: &swarm.Cache_Epoch{Timestamp: 1_700_000_000_000, Signature: seq(0x05, 64)},
		}},
		{"type_swarm_cache_meta", &swarm.CacheMeta{Id: 1, SwarmId: seq(0x01, 32), SwarmSalt: []byte("salt"), Checksum: 0xdeadbeef}},
		{"chat_emote", &chatv1.Emote{
			Id:       1,
			ServerId: 2,
			Name:     "PEPE",
			Images: []*chatv1.EmoteImage{
				{Data: seq(0x00, 10), FileType: chatv1.EmoteFileType_FILE_TYPE_PNG, Height: 28, Width: 28, Scale: chatv1.EmoteScale_EMOTE_SCALE_1X},
				{Data: seq(0x10, 20), FileType: chatv1.EmoteFileType_FILE_TYPE_GIF, Height: 56, Width: 56, Scale: chatv1.EmoteScale_EMOTE_SCALE_2X},
			},
			Effects: []*chatv1.EmoteEffect{
				{Effect: &chatv1.EmoteEffect_CustomCss{CustomCss: &chatv1.EmoteEffect_CustomCSS{StyleSheet: styleSheet(), ExtraWrapCount: 2, WrapAdjacent: true}}},
				{Effect: &chatv1.EmoteEffect_SpriteAnimation_{SpriteAnimation: &chatv1.EmoteEffect_SpriteAnimation{FrameCount: 4, DurationMs: 400, LoopForever: true}}},
				{Effect: &chatv1.EmoteEffect_DefaultModifiers_{DefaultModifiers: &chatv1.EmoteEffect_DefaultModifiers{Modifiers: []string{"wide", "spin"}}}},
				{},
			},
			Contributor: &chatv1.EmoteContributor{Name: "someone", Link: "https://example.com"},
			Labels:      []string{"a", "", "b"},
			Enable:      true,
		}},
		{"chat_modifier", &chatv1.Modifier{
			Id:             3,
			ServerId:       4,
			Name:           "spin",
			Priority:       10,
			Internal:       true,
			ExtraWrapCount: 1,
			ProcChance:     0.125,
			StyleSheet:     styleSheet(),
		}},
		{"directory_listing_snippet", &networkv1directory.ListingSnippet{
			Title:       "title",
			Description: "description",
			Tags:        []string{"one", "two"},
			Category:    "category",
			ChannelName: "channel",
			UserCount:   12345,
			Live:        true,
			IsMature:    false,
			Thumbnail:   &networkv1directory.ListingSnippetImage{SourceOneof: &networkv1directory.ListingSnippetImage_Image{Image: &image.Image{Type: image.ImageType_IMAGE_TYPE_JPEG, Height: 720, Width: 1280, Data: seq(0xff, 16)}}},
			ChannelLogo: &networkv1directory.ListingSnippetImage{SourceOneof: &networkv1directory.ListingSnippetImage_Url{Url: "https://example.com/logo.png"}},
			VideoHeight: 720,
			VideoWidth:  1280,
			ThemeColor:  0xff00ff00,
			StartTime:   -1,
			Key:         seq(0x07, 32),
			Signature:   seq(0x08, 64),
		}},
		{"directory_listing_snippet_delta", &networkv1directory.ListingSnippetDelta{
			Title:       wrapperspb.String(""),
			Description: wrapperspb.String("new description"),
			UserCount:   wrapperspb.UInt64(0),
			Live:        wrapperspb.Bool(false),
			IsMature:    wrapperspb.Bool(true),
			Key:         wrapperspb.Bytes(seq(0x01, 32)),
			ThemeColor:  wrapperspb.UInt32(7),
			StartTime:   wrapperspb.Int64(-100),
			TagsOneof:   &networkv1directory.ListingSnippetDelta_Tags_{Tags: &networkv1directory.ListingSnippetDelta_Tags{Tags: []string{"x"}}},
			ThumbnailOneof: &networkv1directory.ListingSnippetDelta_Thumbnail{
				Thumbnail: &networkv1directory.ListingSnippetImage{},
			},
		}},
		{"devtools_capconn_event", &ppsppv1.CapConnLog_PeerLog_Event{
			Code:             ppsppv1.CapConnLog_PeerLog_Event_EVENT_CODE_READ_ERR,
			Timestamp:        -1_700_000_000_000_000_000,
			MessageTypes:     []ppsppv1.CapConnLog_PeerLog_Event_MessageType{ppsppv1.CapConnLog_PeerLog_Event_MESSAGE_TYPE_DATA, ppsppv1.CapConnLog_PeerLog_Event_MESSAGE_TYPE_END, 0},
			MessageAddresses: []uint64{0, 1, ^uint64(0)},
		}},
	}
}

func main() {
	dir := filepath.Join("crates", "strims-proto", "golden")
	f, err := os.Create(filepath.Join(dir, "vectors.hex"))
	if err != nil {
		panic(err)
	}
	defer f.Close()
	w := bufio.NewWriter(f)
	defer w.Flush()

	write := func(v vector, withStable bool) {
		b, err := proto.MarshalOptions{Deterministic: true}.Marshal(v.msg)
		if err != nil {
			panic(err)
		}
		p := hex.EncodeToString(b)
		if p == "" {
			p = "="
		}
		s := "-"
		if sb, ok := stable(v.msg); withStable && ok {
			s = hex.EncodeToString(sb)
			if s == "" {
				s = "="
			}
		}
		fmt.Fprintf(w, "%s %s %s\n", v.name, p, s)
	}
	for _, v := range vectors() {
		write(v, true)
	}
	for _, v := range rpcVectors() {
		write(v, false)
	}
}
