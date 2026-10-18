// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

package main

import (
	"bufio"
	"bytes"
	"context"
	"encoding/base32"
	"encoding/binary"
	"encoding/hex"
	"errors"
	"fmt"
	"os"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/dao/versionvector"
	authv1 "github.com/MemeLabs/strims/pkg/apis/auth/v1"
	chatv1 "github.com/MemeLabs/strims/pkg/apis/chat/v1"
	daov1 "github.com/MemeLabs/strims/pkg/apis/dao/v1"
	networkv1 "github.com/MemeLabs/strims/pkg/apis/network/v1"
	networkv1bootstrap "github.com/MemeLabs/strims/pkg/apis/network/v1/bootstrap"
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	notificationv1 "github.com/MemeLabs/strims/pkg/apis/notification/v1"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
	replicationv1 "github.com/MemeLabs/strims/pkg/apis/replication/v1"
	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
	"github.com/MemeLabs/strims/pkg/apis/type/image"
	videov1 "github.com/MemeLabs/strims/pkg/apis/video/v1"
	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/MemeLabs/strims/pkg/kv/kvtest"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/types/known/wrapperspb"
)

func u64(v uint64) []byte { return binary.BigEndian.AppendUint64(nil, v) }

func marshal(m proto.Message) []byte {
	return must(proto.MarshalOptions{Deterministic: true}.Marshal(m))
}

func must0(err error) {
	if err != nil {
		panic(err)
	}
}

// storeScenario writes a profile store through the dao API and records
// its kv contents, the records as the dao reads them back, and a
// compressed replication delta.
func storeScenario(w *bufio.Writer) {
	out := func(name string, values ...[]byte) {
		parts := []string{name}
		for _, v := range values {
			if len(v) == 0 {
				parts = append(parts, "=")
			} else {
				parts = append(parts, hex.EncodeToString(v))
			}
		}
		fmt.Fprintln(w, strings.Join(parts, " "))
	}

	blob := kvtest.NewMemStore().(*kvtest.Store)
	out("now", u64(uint64(timeutil.Now().UnixNano())))

	id, key, err := dao.CreateServerAuthThing(blob, "alice", "hunter2")
	must0(err)
	out("auth", []byte("alice"), []byte("hunter2"), u64(id), key)

	sessionKey := seq(200, 32)
	tok := must(dao.CreateSessionThing(blob, sessionKey, id, key))
	out("session", sessionKey, u64(tok.EOL), tok.Token)

	sk := must(dao.NewStorageKeyFromBytes(key, nil))
	ps := dao.NewProfileStore(id, sk, blob, nil)
	rs := must(dao.NewReplicatedStore(ps))
	profile := must(dao.Profile.Get(rs))

	// Networks: one this profile runs, one it joined.
	owned := must(dao.NewNetwork(rs, "owned", nil, profile))
	must0(dao.Networks.Insert(rs, owned))
	otherKey := must(dao.GenerateKey())
	joinedCert := must(dao.NewSelfSignedCertificate(otherKey, certificate.KeyUsage_KEY_USAGE_PEER|certificate.KeyUsage_KEY_USAGE_SIGN, time.Hour, dao.WithSubject("joined")))
	joined := &networkv1.Network{Id: must(rs.GenerateID()), Version: versionvector.New(), Certificate: joinedCert, Alias: "joined"}
	must0(dao.Networks.Insert(rs, joined))
	must(dao.Networks.Transform(rs, joined.Id, func(n *networkv1.Network) error {
		n.Alias = "renamed"
		return nil
	}))

	// The owned network's CA: an invitee, a renewal of the owner's
	// certificate that takes over its subject and key, an alias given up,
	// and the UI's network order.
	peerSign := certificate.KeyUsage_KEY_USAGE_PEER | certificate.KeyUsage_KEY_USAGE_SIGN
	ownerPeer := must(dao.NetworkPeersByPublicKey.Get(rs, dao.FormatNetworkPeerPublicKeyKey(owned.Id, owned.Certificate.Key)))
	inviteeKey := must(dao.GenerateKey())
	inviteeCSR := must(dao.NewCertificateRequest(inviteeKey, peerSign, dao.WithSubject("invitee")))
	inviteeCert := must(dao.SignCertificateRequestWithNetwork(inviteeCSR, owned.ServerConfig))
	must0(dao.CertificateLogs.Insert(rs, must(dao.NewCertificateLog(rs, owned.Id, inviteeCert))))
	must0(dao.NetworkPeers.Insert(rs, must(dao.NewNetworkPeer(rs, owned.Id, inviteeKey.Public, "invitee", ownerPeer.Id))))
	must0(dao.NetworkAliasReservations.Insert(rs, must(dao.NewNetworkAliasReservation(rs, owned.Id, "invitee", inviteeKey.Public))))
	ownerCSR := must(dao.NewCertificateRequest(profile.Key, peerSign, dao.WithSubject(profile.Name)))
	renewed := must(dao.SignCertificateRequestWithNetwork(ownerCSR, owned.ServerConfig))
	must0(dao.CertificateLogs.Insert(rs, must(dao.NewCertificateLog(rs, owned.Id, renewed))))
	spare := must(dao.NewNetworkAliasReservation(rs, owned.Id, "spare", inviteeKey.Public))
	must0(dao.NetworkAliasReservations.Insert(rs, spare))
	must0(dao.NetworkAliasReservations.Release(rs, spare.Id, dao.NetworkAliasReservationCooldown))
	must0(dao.NetworkUIConfig.Set(rs, &networkv1.UIConfig{NetworkDisplayOrder: []uint64{joined.Id, owned.Id}}))

	// Bootstrap clients.
	b1 := must(dao.NewWebSocketBootstrapClient(rs, "wss://one.example/ws", false))
	must0(dao.BootstrapClients.Insert(rs, b1))
	b2 := must(dao.NewWebSocketBootstrapClient(rs, "wss://two.example/ws", true))
	must0(dao.BootstrapClients.Insert(rs, b2))
	must0(dao.BootstrapConfig.Set(rs, &networkv1bootstrap.Config{EnablePublishing: true}))

	// Chat settings.
	must0(dao.ChatUIConfig.Set(rs, &chatv1.UIConfig{Replicate: true, ShowTime: true, Highlight: true}))
	h1 := must(dao.NewChatUIConfigHighlight(rs, "first", []byte("peer-a")))
	must0(dao.ChatUIConfigHighlights.Insert(rs, h1))
	h2 := must(dao.NewChatUIConfigHighlight(rs, "second", []byte("peer-b")))
	must0(dao.ChatUIConfigHighlights.Insert(rs, h2))
	must(dao.ChatUIConfigHighlights.Transform(rs, h1.Id, func(h *chatv1.UIConfigHighlight) error {
		h.Alias = "first renamed"
		return nil
	}))
	must0(dao.ChatUIConfigHighlights.Delete(rs, h2.Id))
	must0(dao.ChatUIConfigTags.Insert(rs, must(dao.NewChatUIConfigTag(rs, "tagged", []byte("peer-c"), "#ff0000"))))
	must0(dao.ChatUIConfigIgnores.Insert(rs, must(dao.NewChatUIConfigIgnore(rs, "ignored", []byte("peer-d"), 0))))

	// Whispers: a second thread with a peer absorbs the first.
	peerKey := must(dao.GenerateKey())
	peerCert := must(dao.NewSelfSignedCertificate(peerKey, certificate.KeyUsage_KEY_USAGE_PEER, time.Hour, dao.WithSubject("peer")))
	t1 := must(dao.NewChatWhisperThread(rs, peerCert))
	t1.LastMessageTime = 100
	t1.HasUnread = true
	must0(dao.ChatWhisperThreads.Insert(rs, t1))
	t2 := must(dao.NewChatWhisperThread(rs, peerCert))
	must0(dao.ChatWhisperThreads.Insert(rs, t2))

	ownCert := &certificate.Certificate{Key: profile.Key.Public, Subject: profile.Name}
	r1 := must(dao.NewChatWhisperRecord(rs, joinedCert.Key, nil, peerCert.Key, peerCert, "hi", nil, 5))
	must0(dao.ChatWhisperRecords.Insert(rs, r1))
	r2 := must(dao.NewChatWhisperRecord(rs, joinedCert.Key, nil, peerCert.Key, ownCert, "hello", nil, 0))
	must0(dao.ChatWhisperRecords.Insert(rs, r2))
	r3 := must(dao.NewChatWhisperRecord(rs, joinedCert.Key, nil, peerCert.Key, peerCert, "again", nil, 6))
	must0(dao.ChatWhisperRecords.Insert(rs, r3))
	must(dao.ChatWhisperRecords.Transform(rs, r3.Id, func(r *chatv1.WhisperRecord) error {
		r.State = chatv1.MessageState_MESSAGE_STATE_READ
		return nil
	}))

	// Devices: one added, one added then deleted.
	phone := must(dao.NewDevice(rs, "phone", "android"))
	must0(dao.Devices.Insert(rs, phone))
	old := must(dao.NewDevice(rs, "old", "windows"))
	must0(dao.Devices.Insert(rs, old))
	must0(dao.Devices.Delete(rs, old.Id))

	// The owned network's directory, whose admin is the owner: a listing
	// of each kind (the media one moderated) and an invitee who may not
	// join.
	swarmID := base32.StdEncoding.WithPadding(base32.NoPadding).EncodeToString(must(dao.GenerateKey()).Public)
	swarmURI := "magnet:?xt=urn:ppspp:" + swarmID + "&x.im=2&x.hf=4&x.sa=2&x.cs=1024&x.ps=64&x.sc=1"
	listings := []*networkv1directory.Listing{
		{Content: &networkv1directory.Listing_Chat_{Chat: &networkv1directory.Listing_Chat{Key: inviteeKey.Public, Name: "lobby"}}},
		{Content: &networkv1directory.Listing_Embed_{Embed: &networkv1directory.Listing_Embed{
			Service: networkv1directory.Listing_Embed_DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_STREAM,
			Id:      "streamer",
		}}},
		{Content: &networkv1directory.Listing_Media_{Media: &networkv1directory.Listing_Media{MimeType: "video/mp4", SwarmUri: swarmURI}}},
		{Content: &networkv1directory.Listing_Service_{Service: &networkv1directory.Listing_Service{Type: "game", SwarmUri: swarmURI}}},
	}
	for _, l := range listings {
		r := must(dao.NewDirectoryListingRecord(rs, owned.Id, l))
		if l.GetMedia() != nil {
			r.Moderation = &networkv1directory.ListingModeration{IsMature: wrapperspb.Bool(true), Category: wrapperspb.String("games")}
			r.Notes = "checked"
		}
		must0(dao.DirectoryListingRecords.Insert(rs, r))
	}
	inviteeUser := must(dao.NewDirectoryUserRecord(rs, owned.Id, inviteeKey.Public))
	inviteeUser.Moderation.DisableJoin = wrapperspb.Bool(true)
	must0(dao.DirectoryUserRecords.Insert(rs, inviteeUser))

	// Chat servers on the owned network, each with an icon, two emotes, a
	// modifier, a tag, and profiles of the invitee (muted) and the owner.
	// The second server is deleted, and its icon with it.
	var servers []*chatv1.Server
	for _, name := range []string{"kept", "deleted"} {
		s := must(dao.NewChatServer(rs, dao.CertificateNetworkKey(owned.Certificate), &chatv1.Room{Name: name, Css: "." + name + " {}"}))
		s.AdminPeerKeys = [][]byte{inviteeKey.Public}
		must0(dao.ChatServers.Insert(rs, s))
		servers = append(servers, s)
		icon := &image.Image{Type: image.ImageType_IMAGE_TYPE_PNG, Height: 1, Width: 1, Data: []byte(name)}
		must0(dao.ChatServerIcons.Insert(rs, &chatv1.ServerIcon{Id: s.Id, Image: icon}))
		for _, emote := range []string{"PeepoHappy", "FeelsBadMan"} {
			images := []*chatv1.EmoteImage{{
				Data:     []byte(emote),
				FileType: chatv1.EmoteFileType_FILE_TYPE_PNG,
				Height:   28,
				Width:    28,
				Scale:    chatv1.EmoteScale_EMOTE_SCALE_2X,
			}}
			effects := []*chatv1.EmoteEffect{{Effect: &chatv1.EmoteEffect_SpriteAnimation_{
				SpriteAnimation: &chatv1.EmoteEffect_SpriteAnimation{FrameCount: 4, DurationMs: 400, LoopForever: true},
			}}}
			contributor := &chatv1.EmoteContributor{Name: "artist", Link: "https://artist.example"}
			must0(dao.ChatEmotes.Insert(rs, must(dao.NewChatEmote(rs, s.Id, emote, images, effects, contributor, []string{"pepe"}, true))))
		}
		must0(dao.ChatModifiers.Insert(rs, must(dao.NewChatModifier(rs, s.Id, "wide", 1, false, 2, 0.25, &chatv1.StyleSheet{Css: ".wide {}"}))))
		must0(dao.ChatTags.Insert(rs, must(dao.NewChatTag(rs, s.Id, "nsfw", "#ff0000", true))))
		muted := must(dao.NewChatProfile(rs, s.Id, inviteeKey.Public, "invitee"))
		muted.Mutes = []*chatv1.Profile_Mute{{CreatedAt: 100, DurationSecs: 60, Message: "spam", ModeratorPeerKey: profile.Key.Public}}
		muted.MuteDeadline = 160
		must0(dao.ChatProfiles.Insert(rs, muted))
		must0(dao.ChatProfiles.Insert(rs, must(dao.NewChatProfile(rs, s.Id, profile.Key.Public, profile.Name))))
	}
	must0(dao.ChatServers.Delete(rs, servers[1].Id))
	must0(dao.ChatServerIcons.Delete(rs, servers[1].Id))

	// Notifications: a failed renewal on the owned network, and one
	// dismissed.
	failed := must(dao.NewNotification(rs, notificationv1.Notification_STATUS_ERROR, "Certificate renewal failed", dao.WithNotificationMessage("rate limited")))
	failed.Subject = &notificationv1.Notification_Subject{Model: notificationv1.Notification_Subject_NOTIFICATION_SUBJECT_MODEL_NETWORK, Id: owned.Id}
	failed.ErrorCode = 2
	must0(dao.Notifications.Insert(rs, failed))
	dismissed := must(dao.NewNotification(rs, notificationv1.Notification_STATUS_INFO, "dismissed"))
	must0(dao.Notifications.Insert(rs, dismissed))
	must0(dao.Notifications.Delete(rs, dismissed.Id))

	// Video channels: the profile's own on the owned network, the
	// invitee's share of it, a remote share on the joined network, and a
	// deleted one. Both configs, the ingress one naming both networks.
	insertChannel := func(c *videov1.VideoChannel, title string) {
		c.DirectoryListingSnippet = &networkv1directory.ListingSnippet{Title: title, Description: title + " stream", Tags: []string{"video"}, Category: "games", ChannelName: title, IsMature: true}
		must0(dao.VideoChannels.Insert(rs, c))
	}
	ownedKey := dao.CertificateNetworkKey(owned.Certificate)
	localChannel := must(dao.NewVideoChannel(rs))
	localChannel.Owner = &videov1.VideoChannel_Local_{Local: &videov1.VideoChannel_Local{AuthKey: profile.Key.Public, NetworkKey: ownedKey}}
	insertChannel(localChannel, "local")
	shareChannel := must(dao.NewVideoChannel(rs))
	shareChannel.Owner = &videov1.VideoChannel_LocalShare_{LocalShare: &videov1.VideoChannel_LocalShare{Certificate: inviteeCert}}
	insertChannel(shareChannel, "share")
	remoteChannel := must(dao.NewVideoChannel(rs))
	remoteChannel.Owner = &videov1.VideoChannel_RemoteShare_{RemoteShare: &videov1.VideoChannel_RemoteShare{
		Id:          42,
		NetworkKey:  joinedCert.Key,
		ServiceKey:  must(dao.GenerateKey()).Public,
		ServiceSalt: []byte("salt"),
		ServerAddr:  "rtmp.example:1935",
	}}
	insertChannel(remoteChannel, "remote")
	deletedChannel := must(dao.NewVideoChannel(rs))
	deletedChannel.Owner = &videov1.VideoChannel_Local_{Local: &videov1.VideoChannel_Local{AuthKey: profile.Key.Public, NetworkKey: ownedKey}}
	insertChannel(deletedChannel, "deleted")
	must0(dao.VideoChannels.Delete(rs, deletedChannel.Id))
	must0(dao.VideoIngressConfig.Set(rs, &videov1.VideoIngressConfig{
		Enabled:            true,
		ServerAddr:         "0.0.0.0:1935",
		PublicServerAddr:   "rtmp.example:1935",
		ServiceNetworkKeys: [][]byte{ownedKey, joinedCert.Key},
	}))
	must0(dao.HLSEgressConfig.Set(rs, &videov1.HLSEgressConfig{Enabled: true, PublicServerAddr: "https://hls.example"}))
	out("video_stream_key", u64(localChannel.Id), []byte(dao.FormatVideoChannelStreamKey(localChannel.Id, localChannel.Token, profile.Key)))
	out("video_stream_key", u64(shareChannel.Id), []byte(dao.FormatVideoChannelStreamKey(shareChannel.Id, shareChannel.Token, inviteeKey)))

	_, err = dao.NewMutex(zap.NewNop(), rs, "replication").TryLock(context.Background())
	must0(err)

	for table, entries := range blob.Dump() {
		keys := make([]string, 0, len(entries))
		for k := range entries {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		for _, k := range keys {
			out("kv", []byte(table), []byte(k), entries[k])
		}
	}

	// The records as the dao reads them.
	record := func(ns uint64, id uint64, m proto.Message) { out("record", u64(ns), u64(id), marshal(m)) }
	record(2001, 0, profile)
	record(2002, 0, profileIDs(rs))
	record(6008, 0, must(dao.ChatUIConfig.Get(rs)))
	for _, m := range must(dao.Devices.GetAll(rs)) {
		record(2003, m.Id, m)
	}
	for _, m := range must(dao.Networks.GetAll(rs)) {
		record(4001, m.Id, m)
	}
	for _, m := range must(dao.BootstrapClients.GetAll(rs)) {
		record(4007, m.Id, m)
	}
	record(4008, 0, must(dao.NetworkUIConfig.Get(rs)))
	record(4015, 0, must(dao.BootstrapConfig.Get(rs)))
	for _, m := range must(dao.CertificateLogs.GetAll(rs)) {
		record(4003, m.Id, m)
	}
	for _, m := range must(dao.NetworkPeers.GetAll(rs)) {
		record(4009, m.Id, m)
	}
	for _, m := range must(dao.NetworkAliasReservations.GetAll(rs)) {
		record(4016, m.Id, m)
	}
	for _, m := range must(dao.ChatUIConfigHighlights.GetAll(rs)) {
		record(6018, m.Id, m)
	}
	for _, m := range must(dao.ChatUIConfigTags.GetAll(rs)) {
		record(6020, m.Id, m)
	}
	for _, m := range must(dao.ChatUIConfigIgnores.GetAll(rs)) {
		record(6022, m.Id, m)
	}
	for _, m := range must(dao.ChatWhisperThreads.GetAll(rs)) {
		record(6015, m.Id, m)
	}
	for _, m := range must(dao.ChatWhisperRecords.GetAll(rs)) {
		record(6012, m.Id, m)
	}
	for _, m := range must(dao.ReplicationEventLogs.GetAll(rs)) {
		record(1003, m.Id, m)
	}
	for _, m := range must(dao.ReplicationCheckpoints.GetAll(rs)) {
		record(1004, m.Id, m)
	}
	for _, m := range must(dao.DirectoryListingRecords.GetAll(rs)) {
		record(9001, m.Id, m)
	}
	for _, m := range must(dao.DirectoryUserRecords.GetAll(rs)) {
		record(9004, m.Id, m)
	}
	for _, m := range must(dao.ChatServers.GetAll(rs)) {
		record(6001, m.Id, m)
	}
	for _, m := range must(dao.ChatServerIcons.GetAll(rs)) {
		record(6024, m.Id, m)
	}
	for _, m := range must(dao.ChatEmotes.GetAll(rs)) {
		record(6002, m.Id, m)
	}
	for _, m := range must(dao.ChatModifiers.GetAll(rs)) {
		record(6004, m.Id, m)
	}
	for _, m := range must(dao.ChatTags.GetAll(rs)) {
		record(6006, m.Id, m)
	}
	for _, m := range must(dao.ChatProfiles.GetAll(rs)) {
		record(6009, m.Id, m)
	}
	for _, m := range must(dao.Notifications.GetAll(rs)) {
		record(5001, m.Id, m)
	}
	for _, m := range must(dao.VideoChannels.GetAll(rs)) {
		record(7001, m.Id, m)
	}
	record(7003, 0, must(dao.VideoIngressConfig.Get(rs)))
	record(7004, 0, must(dao.HLSEgressConfig.Get(rs)))

	// A peer that has seen the first three of this replica's logs.
	remote := &daov1.VersionVector{Value: map[uint64]uint64{rs.ReplicaID(): 3}}
	out("delta_remote", marshal(remote))
	for _, l := range must(dao.ReplicationEventLogs.GetCompressedDelta(rs, remote)) {
		out("delta", marshal(l))
	}
}

// verify checks a store another implementation wrote, as described by
// the lines of path, and exits non-zero on the first mismatch.
func verify(path string) {
	f := must(os.Open(path))
	defer f.Close()

	blob := kvtest.NewMemStore()
	type check struct {
		kind   string
		values [][]byte
	}
	var checks []check
	scanner := bufio.NewScanner(f)
	scanner.Buffer(make([]byte, 1<<20), 1<<26)
	for scanner.Scan() {
		parts := strings.Split(scanner.Text(), " ")
		values := make([][]byte, len(parts)-1)
		for i, p := range parts[1:] {
			if p != "=" {
				values[i] = must(hex.DecodeString(p))
			}
		}
		if parts[0] == "kv" {
			table := string(values[0])
			must0(blob.CreateStoreIfNotExists(table))
			must0(blob.Update(table, func(tx kv.BlobTx) error { return tx.Put(string(values[1]), values[2]) }))
			continue
		}
		checks = append(checks, check{parts[0], values})
	}
	must0(scanner.Err())

	fail := func(format string, args ...any) {
		fmt.Fprintf(os.Stderr, "verify: "+format+"\n", args...)
		os.Exit(1)
	}
	var rs *dao.ReplicatedStore
	for _, c := range checks {
		v := c.values
		switch c.kind {
		case "auth":
			id, key, err := dao.LoadServerAuthThing(blob, string(v[0]), string(v[1]))
			must0(err)
			if id != binary.BigEndian.Uint64(v[2]) || !bytes.Equal(key, v[3]) {
				fail("auth: got profile %d", id)
			}
			sk := must(dao.NewStorageKeyFromBytes(key, nil))
			rs = must(dao.NewReplicatedStore(dao.NewProfileStore(id, sk, blob, nil)))
		case "session":
			id, key, err := dao.LoadSessionThing(blob, v[0], &dao.SessionToken{EOL: binary.BigEndian.Uint64(v[1]), Token: v[2]})
			must0(err)
			if id != binary.BigEndian.Uint64(v[3]) || !bytes.Equal(key, v[4]) {
				fail("session: got profile %d", id)
			}
		case "record":
			ns, id := binary.BigEndian.Uint64(v[0]), binary.BigEndian.Uint64(v[1])
			got := getRecord(rs, ns, id)
			want := got.ProtoReflect().New().Interface()
			must0(proto.Unmarshal(v[2], want))
			if !proto.Equal(got, want) {
				fail("record %d/%d: got %v, want %v", ns, id, got, want)
			}
		case "absent":
			ns, id := binary.BigEndian.Uint64(v[0]), binary.BigEndian.Uint64(v[1])
			if _, err := deletableRecord(rs, ns, id); !errors.Is(err, kv.ErrRecordNotFound) {
				fail("record %d/%d: want none, got error %v", ns, id, err)
			}
		case "index":
			ns := binary.BigEndian.Uint64(v[0])
			var want []uint64
			for i := 0; i < len(v[2]); i += 8 {
				want = append(want, binary.BigEndian.Uint64(v[2][i:]))
			}
			got := indexIDs(rs, ns, v[1])
			if fmt.Sprint(got) != fmt.Sprint(want) {
				fail("index %d %x: got %v, want %v", ns, v[1], got, want)
			}
		case "stream_key":
			// The channel a stream key names, or none.
			c, err := dao.GetVideoChannelByStreamKey(rs, string(v[0]))
			switch {
			case len(v[1]) == 0 && err == nil:
				fail("stream key %s: want none, got channel %d", v[0], c.Id)
			case len(v[1]) != 0 && err != nil:
				fail("stream key %s: %v", v[0], err)
			case len(v[1]) != 0 && c.Id != binary.BigEndian.Uint64(v[1]):
				fail("stream key %s: got channel %d, want %d", v[0], c.Id, binary.BigEndian.Uint64(v[1]))
			}
		case "apply":
			// Logs from the store's replica, applied to a new replica of
			// the same profile.
			var logs []*replicationv1.EventLog
			for _, b := range v[1:] {
				l := &replicationv1.EventLog{}
				must0(proto.Unmarshal(b, l))
				logs = append(logs, l)
			}
			target := newReplica(rs, binary.BigEndian.Uint64(v[0]))
			_, err := dao.ApplyReplicationEventLogs(target, logs)
			must0(err)
			for _, m := range must(dao.ChatUIConfigHighlights.GetAll(rs)) {
				got := must(dao.ChatUIConfigHighlights.Get(target, m.Id))
				if got.Alias != m.Alias {
					fail("applied highlight %d: got %q, want %q", m.Id, got.Alias, m.Alias)
				}
			}
		case "pairing":
			tok := &authv1.PairingToken{}
			must0(proto.Unmarshal(v[0], tok))
			paired := kvtest.NewMemStore()
			must0(dao.ImportPairingToken(paired, tok, v[1]))
			id, _, err := dao.LoadServerAuthThing(paired, tok.Auth.Name, string(v[2]))
			must0(err)
			sk := must(dao.NewStorageKeyFromBytes(v[1], nil))
			p := must(dao.Profile.Get(dao.NewProfileStore(id, sk, paired, nil)))
			if p.DeviceId != tok.Profile.DeviceId {
				fail("pairing: got device %d", p.DeviceId)
			}
		default:
			fail("unknown check %q", c.kind)
		}
	}
	fmt.Println("ok")
}

// profileIDs reads the id range record, which the dao exposes only
// through its allocator.
func profileIDs(s kv.Store) *profilev1.ProfileID {
	p := &profilev1.ProfileID{}
	must0(s.View(func(tx kv.Tx) error { return tx.Get(strconv.FormatInt(2002, 36), p) }))
	return p
}

// newReplica creates a store for another device of rs's profile.
func newReplica(rs *dao.ReplicatedStore, device uint64) *dao.ReplicatedStore {
	profile := proto.Clone(must(dao.Profile.Get(rs))).(*profilev1.Profile)
	profile.DeviceId = device
	blob := kvtest.NewMemStore()
	user := &authv1.ServerUserThing{Name: profile.Name + "-replica"}
	ps := must(dao.StoreProfileThing(blob, user, profile, rs.Key().Key()))
	must0(dao.ProfileID.Init(ps, &profilev1.ProfileID{NextId: device << 32, LastId: (device + 1) << 32}))
	return must(dao.NewReplicatedStore(ps))
}

func getRecord(s kv.Store, ns, id uint64) proto.Message {
	switch ns {
	case 2001:
		return must(dao.Profile.Get(s))
	case 2002:
		return profileIDs(s)
	case 6008:
		return must(dao.ChatUIConfig.Get(s))
	case 2003:
		return must(dao.Devices.Get(s, id))
	case 4001:
		return must(dao.Networks.Get(s, id))
	case 4007:
		return must(dao.BootstrapClients.Get(s, id))
	case 4008:
		return must(dao.NetworkUIConfig.Get(s))
	case 4015:
		return must(dao.BootstrapConfig.Get(s))
	case 4003:
		return must(dao.CertificateLogs.Get(s, id))
	case 4009:
		return must(dao.NetworkPeers.Get(s, id))
	case 4016:
		return must(dao.NetworkAliasReservations.Get(s, id))
	case 6018:
		return must(dao.ChatUIConfigHighlights.Get(s, id))
	case 6020:
		return must(dao.ChatUIConfigTags.Get(s, id))
	case 6022:
		return must(dao.ChatUIConfigIgnores.Get(s, id))
	case 6015:
		return must(dao.ChatWhisperThreads.Get(s, id))
	case 6012:
		return must(dao.ChatWhisperRecords.Get(s, id))
	case 1003:
		return must(dao.ReplicationEventLogs.Get(s, id))
	case 1004:
		return must(dao.ReplicationCheckpoints.Get(s, id))
	case 9001:
		return must(dao.DirectoryListingRecords.Get(s, id))
	case 9004:
		return must(dao.DirectoryUserRecords.Get(s, id))
	case 7003:
		return must(dao.VideoIngressConfig.Get(s))
	case 7004:
		return must(dao.HLSEgressConfig.Get(s))
	}
	return must(deletableRecord(s, ns, id))
}

// deletableRecord reads record id of a table whose deletions the checks
// cover: the chat server tables, notifications and video channels.
func deletableRecord(s kv.Store, ns, id uint64) (proto.Message, error) {
	switch ns {
	case 5001:
		return dao.Notifications.Get(s, id)
	case 7001:
		return dao.VideoChannels.Get(s, id)
	case 6001:
		return dao.ChatServers.Get(s, id)
	case 6024:
		return dao.ChatServerIcons.Get(s, id)
	case 6002:
		return dao.ChatEmotes.Get(s, id)
	case 6004:
		return dao.ChatModifiers.Get(s, id)
	case 6006:
		return dao.ChatTags.Get(s, id)
	case 6009:
		return dao.ChatProfiles.Get(s, id)
	}
	panic(fmt.Sprintf("unknown namespace %d", ns))
}

func ids[T interface{ GetId() uint64 }](ms []T, err error) []uint64 {
	must0(err)
	out := []uint64{}
	for _, m := range ms {
		out = append(out, m.GetId())
	}
	sort.Slice(out, func(i, j int) bool { return out[i] < out[j] })
	return out
}

func sortedIDs(ids []uint64, err error) []uint64 {
	must0(err)
	out := append([]uint64{}, ids...)
	sort.Slice(out, func(i, j int) bool { return out[i] < out[j] })
	return out
}

func one[T interface{ GetId() uint64 }](m T, err error) []uint64 {
	if errors.Is(err, kv.ErrRecordNotFound) {
		return []uint64{}
	}
	must0(err)
	return []uint64{m.GetId()}
}

func indexIDs(s kv.Store, ns uint64, key []byte) []uint64 {
	switch ns {
	case 4002:
		return one(dao.NetworksByKey.Get(s, key))
	case 4014:
		return one(dao.BootstrapClientsByClientOptions.Get(s, key))
	case 4004:
		return ids(dao.CertificateLogsByNetwork.GetAllByRefID(s, binary.LittleEndian.Uint64(key)))
	case 4005:
		return one(dao.CertificateLogsBySerialNumber.Get(s, key))
	case 4006:
		return one(dao.CertificateLogsBySubject.Get(s, key))
	case 4013:
		return one(dao.CertificateLogsByKey.Get(s, key))
	case 4010:
		return ids(dao.NetworkPeersByNetwork.GetAllByRefID(s, binary.LittleEndian.Uint64(key)))
	case 4011:
		return one(dao.NetworkPeersByPublicKey.Get(s, key))
	case 4012:
		return ids(dao.NetworkPeersByInviterPeer.GetAllByRefID(s, binary.LittleEndian.Uint64(key)))
	case 4017:
		return ids(dao.NetworkAliasReservationsByNetwork.GetAllByRefID(s, binary.LittleEndian.Uint64(key)))
	case 4018:
		return one(dao.NetworkAliasReservationsByAlias.Get(s, key))
	case 6019:
		return one(dao.ChatUIConfigHighlightsByPeerKey.Get(s, key))
	case 6021:
		return one(dao.ChatUIConfigTagsByPeerKey.Get(s, key))
	case 6023:
		return one(dao.ChatUIConfigIgnoresByPeerKey.Get(s, key))
	case 6016:
		return one(dao.ChatWhisperThreadsByPeerKey.Get(s, key))
	case 6013:
		return ids(dao.ChatWhisperRecordsByPeerKey.GetAll(s, key))
	case 6017:
		return ids(dao.UnreadChatWhisperRecordsByPeerKey.GetAll(s, key))
	case 6014:
		return ids(dao.ChatWhisperRecordsByState.GetAll(s, key))
	case 9002:
		return ids(dao.DirectoryListingRecordsByNetwork.GetAllByRefID(s, binary.LittleEndian.Uint64(key)))
	case 9003:
		return one(dao.DirectoryListingRecordsByListing.Get(s, key))
	case 9005:
		return one(dao.DirectoryUserRecordsByPeerKey.Get(s, key))
	case 9006:
		return ids(dao.DirectoryUserRecordsByNetwork.GetAllByRefID(s, binary.LittleEndian.Uint64(key)))
	case 6003:
		return ids(dao.ChatEmotesByServer.GetAllByRefID(s, binary.LittleEndian.Uint64(key)))
	case 6005:
		return ids(dao.ChatModifiersByServer.GetAllByRefID(s, binary.LittleEndian.Uint64(key)))
	case 6007:
		return ids(dao.ChatTagsByServer.GetAllByRefID(s, binary.LittleEndian.Uint64(key)))
	case 6010:
		return ids(dao.ChatProfilesByServer.GetAllByRefID(s, binary.LittleEndian.Uint64(key)))
	case 6011:
		return one(dao.ChatProfilesByPeerKey.Get(s, key))
	case 7002:
		return sortedIDs(dao.ScanSecondaryIndex(s, 7002, key))
	}
	panic(fmt.Sprintf("unknown index %d", ns))
}
