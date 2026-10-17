// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

package main

// internal/chat on the node, for the chat interop tests. With -chat (and
// -directory), internal/chat's control runs on the helper's profile store,
// beside the network, transfer and directory controls, as internal/app
// builds it: it serves the chat servers the profile owns, reads the
// others', and runs the whisper runner, which serves the profile's
// whisper address on every network and delivers its enqueued whispers.
// The verbs write records through the dao as internal/frontend writes
// them.
//
// Verbs (byte strings in hex, <key> a network key, <server key> a chat
// server's public key, <id> a record id and <secs> seconds in decimal,
// <room>, <image>, <emote>, <modifier> and <tag> encoded Room, Image,
// Emote, Modifier and Tag messages):
//
//	chat-server-create <key> <room>                  a Server with a new key -> <id> <server key>
//	chat-server-update <id> <key> <room> <admins>    set the Server's network key, Room and admin peer keys
//	                                                 (comma-separated, or - for none)
//	chat-server-delete <id>                          delete the Server and its icon; its emotes, modifiers,
//	                                                 tags and profiles go with it
//	chat-server <id>                                 -> <Server>
//	chat-icon-set <id> <image>                       set the server's ServerIcon
//	chat-icon <id>                                   -> <ServerIcon>, or - for none
//	chat-emote-create <emote>                        an Emote with the emote's server_id, name, images,
//	                                                 effects, contributor, labels and enable -> <id>
//	chat-emote-update <emote>                        set those fields on the Emote with the emote's id
//	chat-emote-delete <id>
//	chat-modifier-create <modifier>                  a Modifier with the modifier's server_id, name,
//	                                                 priority, internal, extra_wrap_count, proc_chance and
//	                                                 style_sheet -> <id>
//	chat-modifier-update <modifier>                  set those fields on the Modifier with the modifier's id
//	chat-modifier-delete <id>
//	chat-tag-create <tag>                            a Tag with the tag's server_id, name, color and
//	                                                 sensitive -> <id>
//	chat-tag-update <tag>                            set those fields on the Tag with the tag's id
//	chat-tag-delete <id>
//	chat-emotes <id>                                 the server's records -> <Emote>,..., or - for none
//	chat-modifiers <id>                              -> <Modifier>,..., or - for none
//	chat-tags <id>                                   -> <Tag>,..., or - for none
//	chat-profiles <id>                               -> <Profile>,..., or - for none
//	chat-sync <id> <unified 0|1>                     write the server's asset bundle, unified or as a delta
//	chat-read <key> <server key>                     read the server's events and asset bundles until
//	                                                 chat-read-stop; one read per server key
//	chat-read-stop <server key>
//	chat-send <key> <server key> <body>
//	chat-mute <key> <server key> <peer key> <secs> <message>
//	chat-unmute <key> <server key> <peer key>
//	chat-get-mute <key> <server key>                 -> <end time> <message>
//	chat-whisper <key> <peer key> <alias> <body>     a whisper to the member with the peer key, or with the
//	                                                 alias when the key is -: an ENQUEUED WhisperRecord and
//	                                                 its thread -> <record id>
//	chat-whispers <peer key>                         the thread with the peer -> <ListWhispersResponse>
//	chat-whispers-read <peer key>                    mark the thread and its UNREAD records read
//	chat-ignore <peer key> <alias> <secs>            ignore the peer until now + secs (signed), or with no
//	                                                 deadline when secs is -; replaces the peer's ignore
//	chat-unignore <peer key>
//	chat-ignores                                     -> <UIConfigIgnore>,..., or - for none
//
// Events:
//
//	chat-event <server key> <ServerEvent>     a read's event
//	chat-assets <server key> <AssetBundle>    a read's asset bundle
//	chat-read-end <server key>                a read ended
//	whisper-record <WhisperRecord>            a WhisperRecord was written
//	whisper-thread <WhisperThread>            a WhisperThread was written
//
// A call that fails answers err with the error's message.

import (
	"context"
	"encoding/hex"
	"errors"
	"fmt"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/MemeLabs/strims/internal/chat"
	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/event"
	chatv1 "github.com/MemeLabs/strims/pkg/apis/chat/v1"
	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
	"github.com/MemeLabs/strims/pkg/apis/type/image"
	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"go.uber.org/zap"
	"google.golang.org/protobuf/proto"
)

// chatCallTimeout bounds each call to a chat server and each certificate
// lookup. It is shorter than the test's step, so a call that fails
// answers err in time for the test to retry it.
const chatCallTimeout = 10 * time.Second

type chatState struct {
	control chat.Control

	mu sync.Mutex
	// The open reads, by hex server key.
	reads map[string]*chatRead
}

type chatRead struct {
	cancel context.CancelFunc
}

// newChatState runs internal/chat's control on the network state's store
// and controls, as internal/app builds it.
func newChatState(ctx context.Context, logger *zap.Logger, s *networkState) *chatState {
	c := &chatState{
		control: chat.NewControl(ctx, logger, s.store, s.observers, s.profile, s.control, s.transfer, s.directory),
		reads:   map[string]*chatRead{},
	}
	go c.control.Run()
	go c.report(s.observers)
	return c
}

// report emits the whisper records and threads the store writes. The
// observers fail a channel that blocks for a second, so it only emits.
func (c *chatState) report(obs *event.Observers) {
	events, done := obs.Events()
	defer done()
	for e := range events {
		switch e := e.(type) {
		case *chatv1.WhisperRecordChangeEvent:
			emitMessage("whisper-record", e.WhisperRecord)
		case *chatv1.WhisperThreadChangeEvent:
			emitMessage("whisper-thread", e.WhisperThread)
		}
	}
}

// emitMessage emits "<prefix> <m hex>".
func emitMessage(prefix string, m proto.Message) {
	b, err := proto.Marshal(m)
	if err != nil {
		logf("%s: %v", prefix, err)
		return
	}
	emit("%s %s", prefix, hexs(string(b)))
}

func messageHex(m proto.Message) (string, error) {
	b, err := proto.Marshal(m)
	if err != nil {
		return "", err
	}
	return hexs(string(b)), nil
}

// messageList answers records as comma-separated hex, or - for none.
func messageList[T proto.Message](ms []T) (string, error) {
	if len(ms) == 0 {
		return "-", nil
	}
	parts := make([]string, len(ms))
	for i, m := range ms {
		h, err := messageHex(m)
		if err != nil {
			return "", err
		}
		parts[i] = h
	}
	return strings.Join(parts, ","), nil
}

// read emits a read's events and asset bundles until both channels close.
func (c *chatState) read(k string, r *chatRead, events <-chan *chatv1.ServerEvent, assets <-chan *chatv1.AssetBundle) {
	for events != nil || assets != nil {
		select {
		case e, ok := <-events:
			if !ok {
				events = nil
				continue
			}
			emitMessage("chat-event "+k, e)
		case b, ok := <-assets:
			if !ok {
				assets = nil
				continue
			}
			emitMessage("chat-assets "+k, b)
		}
	}
	c.endRead(k, r)
	emit("chat-read-end %s", k)
}

// endRead forgets the read, unless another took its key, and cancels it.
func (c *chatState) endRead(k string, r *chatRead) {
	c.mu.Lock()
	if c.reads[k] == r {
		delete(c.reads, k)
	}
	c.mu.Unlock()
	r.cancel()
}

func (n *node) chatCommand(verb string, args []string) (string, error) {
	s := n.network
	if s == nil || s.chat == nil {
		return "", fmt.Errorf("%s: chat is off", verb)
	}
	c := s.chat
	store := s.store
	need := func(k int) error {
		if len(args) < k {
			return fmt.Errorf("%s: want %d args, got %d", verb, k, len(args))
		}
		return nil
	}
	bytesArg := func(i int) ([]byte, error) {
		if err := need(i + 1); err != nil {
			return nil, err
		}
		return unhex(args[i])
	}
	stringArg := func(i int) (string, error) {
		b, err := bytesArg(i)
		return string(b), err
	}
	uintArg := func(i int) (uint64, error) {
		if err := need(i + 1); err != nil {
			return 0, err
		}
		return strconv.ParseUint(args[i], 10, 64)
	}
	message := func(i int, m proto.Message) error {
		b, err := bytesArg(i)
		if err != nil {
			return err
		}
		return proto.Unmarshal(b, m)
	}
	// networkServer is the <key> <server key> pair at i.
	networkServer := func(i int) ([]byte, []byte, error) {
		key, err := bytesArg(i)
		if err != nil {
			return nil, nil, err
		}
		serverKey, err := bytesArg(i + 1)
		return key, serverKey, err
	}
	ctx, cancel := context.WithTimeout(context.Background(), chatCallTimeout)
	defer cancel()

	switch verb {
	case "chat-server-create":
		key, err := bytesArg(0)
		if err != nil {
			return "", err
		}
		room := &chatv1.Room{}
		if err := message(1, room); err != nil {
			return "", err
		}
		server, err := dao.NewChatServer(store, key, room)
		if err != nil {
			return "", err
		}
		if err := dao.ChatServers.Insert(store, server); err != nil {
			return "", err
		}
		return fmt.Sprintf("%d %s", server.Id, hex.EncodeToString(server.Key.Public)), nil

	case "chat-server-update":
		id, err := uintArg(0)
		if err != nil {
			return "", err
		}
		key, err := bytesArg(1)
		if err != nil {
			return "", err
		}
		room := &chatv1.Room{}
		if err := message(2, room); err != nil {
			return "", err
		}
		if err := need(4); err != nil {
			return "", err
		}
		var admins [][]byte
		if args[3] != "-" {
			for _, a := range strings.Split(args[3], ",") {
				k, err := hex.DecodeString(a)
				if err != nil {
					return "", err
				}
				admins = append(admins, k)
			}
		}
		_, err = dao.ChatServers.Transform(store, id, func(v *chatv1.Server) error {
			v.NetworkKey = key
			v.Room = room
			v.AdminPeerKeys = admins
			return nil
		})
		return "", err

	case "chat-server-delete":
		id, err := uintArg(0)
		if err != nil {
			return "", err
		}
		return "", store.Update(func(tx kv.RWTx) error {
			if err := dao.ChatServers.Delete(tx, id); err != nil {
				return err
			}
			err := dao.ChatServerIcons.Delete(tx, id)
			if errors.Is(err, kv.ErrRecordNotFound) {
				return nil
			}
			return err
		})

	case "chat-server":
		id, err := uintArg(0)
		if err != nil {
			return "", err
		}
		server, err := dao.ChatServers.Get(store, id)
		if err != nil {
			return "", err
		}
		return messageHex(server)

	case "chat-icon-set":
		id, err := uintArg(0)
		if err != nil {
			return "", err
		}
		img := &image.Image{}
		if err := message(1, img); err != nil {
			return "", err
		}
		return "", dao.ChatServerIcons.Upsert(store, &chatv1.ServerIcon{Id: id, Image: img})

	case "chat-icon":
		id, err := uintArg(0)
		if err != nil {
			return "", err
		}
		icon, err := dao.ChatServerIcons.Get(store, id)
		if errors.Is(err, kv.ErrRecordNotFound) {
			return "-", nil
		} else if err != nil {
			return "", err
		}
		return messageHex(icon)

	case "chat-emote-create", "chat-emote-update":
		e := &chatv1.Emote{}
		if err := message(0, e); err != nil {
			return "", err
		}
		if verb == "chat-emote-update" {
			_, err := dao.ChatEmotes.Transform(store, e.Id, func(v *chatv1.Emote) error {
				v.Name = e.Name
				v.Images = e.Images
				v.Contributor = e.Contributor
				v.Effects = e.Effects
				v.Labels = e.Labels
				v.Enable = e.Enable
				return nil
			})
			return "", err
		}
		emote, err := dao.NewChatEmote(store, e.ServerId, e.Name, e.Images, e.Effects, e.Contributor, e.Labels, e.Enable)
		if err != nil {
			return "", err
		}
		if err := dao.ChatEmotes.Insert(store, emote); err != nil {
			return "", err
		}
		return strconv.FormatUint(emote.Id, 10), nil

	case "chat-modifier-create", "chat-modifier-update":
		m := &chatv1.Modifier{}
		if err := message(0, m); err != nil {
			return "", err
		}
		if verb == "chat-modifier-update" {
			_, err := dao.ChatModifiers.Transform(store, m.Id, func(v *chatv1.Modifier) error {
				v.Name = m.Name
				v.Priority = m.Priority
				v.Internal = m.Internal
				v.ExtraWrapCount = m.ExtraWrapCount
				v.ProcChance = m.ProcChance
				v.StyleSheet = m.StyleSheet
				return nil
			})
			return "", err
		}
		modifier, err := dao.NewChatModifier(store, m.ServerId, m.Name, m.Priority, m.Internal, m.ExtraWrapCount, m.ProcChance, m.StyleSheet)
		if err != nil {
			return "", err
		}
		if err := dao.ChatModifiers.Insert(store, modifier); err != nil {
			return "", err
		}
		return strconv.FormatUint(modifier.Id, 10), nil

	case "chat-tag-create", "chat-tag-update":
		t := &chatv1.Tag{}
		if err := message(0, t); err != nil {
			return "", err
		}
		if verb == "chat-tag-update" {
			_, err := dao.ChatTags.Transform(store, t.Id, func(v *chatv1.Tag) error {
				v.Name = t.Name
				v.Color = t.Color
				v.Sensitive = t.Sensitive
				return nil
			})
			return "", err
		}
		tag, err := dao.NewChatTag(store, t.ServerId, t.Name, t.Color, t.Sensitive)
		if err != nil {
			return "", err
		}
		if err := dao.ChatTags.Insert(store, tag); err != nil {
			return "", err
		}
		return strconv.FormatUint(tag.Id, 10), nil

	case "chat-emote-delete", "chat-modifier-delete", "chat-tag-delete":
		id, err := uintArg(0)
		if err != nil {
			return "", err
		}
		switch verb {
		case "chat-emote-delete":
			return "", dao.ChatEmotes.Delete(store, id)
		case "chat-modifier-delete":
			return "", dao.ChatModifiers.Delete(store, id)
		}
		return "", dao.ChatTags.Delete(store, id)

	case "chat-emotes", "chat-modifiers", "chat-tags", "chat-profiles":
		id, err := uintArg(0)
		if err != nil {
			return "", err
		}
		switch verb {
		case "chat-emotes":
			ms, err := dao.ChatEmotesByServer.GetAllByRefID(store, id)
			if err != nil {
				return "", err
			}
			return messageList(ms)
		case "chat-modifiers":
			ms, err := dao.ChatModifiersByServer.GetAllByRefID(store, id)
			if err != nil {
				return "", err
			}
			return messageList(ms)
		case "chat-tags":
			ms, err := dao.ChatTagsByServer.GetAllByRefID(store, id)
			if err != nil {
				return "", err
			}
			return messageList(ms)
		}
		ms, err := dao.ChatProfilesByServer.GetAllByRefID(store, id)
		if err != nil {
			return "", err
		}
		return messageList(ms)

	case "chat-sync":
		id, err := uintArg(0)
		if err != nil {
			return "", err
		}
		if err := need(2); err != nil {
			return "", err
		}
		return "", c.control.SyncAssets(id, args[1] == "1")

	case "chat-read":
		key, serverKey, err := networkServer(0)
		if err != nil {
			return "", err
		}
		k := hex.EncodeToString(serverKey)
		rctx, rcancel := context.WithCancel(context.Background())
		r := &chatRead{cancel: rcancel}
		c.mu.Lock()
		if _, ok := c.reads[k]; ok {
			c.mu.Unlock()
			rcancel()
			return "", errors.New("already reading")
		}
		c.reads[k] = r
		c.mu.Unlock()
		events, assets, err := c.control.ReadServer(rctx, key, serverKey)
		if err != nil {
			c.endRead(k, r)
			return "", err
		}
		go c.read(k, r, events, assets)
		return "", nil

	case "chat-read-stop":
		serverKey, err := bytesArg(0)
		if err != nil {
			return "", err
		}
		k := hex.EncodeToString(serverKey)
		c.mu.Lock()
		r, ok := c.reads[k]
		delete(c.reads, k)
		c.mu.Unlock()
		if !ok {
			return "", errors.New("not reading")
		}
		r.cancel()
		return "", nil

	case "chat-send":
		key, serverKey, err := networkServer(0)
		if err != nil {
			return "", err
		}
		body, err := stringArg(2)
		if err != nil {
			return "", err
		}
		return "", c.control.SendMessage(ctx, key, serverKey, body)

	case "chat-mute", "chat-unmute":
		key, serverKey, err := networkServer(0)
		if err != nil {
			return "", err
		}
		peerKey, err := bytesArg(2)
		if err != nil {
			return "", err
		}
		if verb == "chat-unmute" {
			return "", c.control.Unmute(ctx, key, serverKey, peerKey)
		}
		if err := need(4); err != nil {
			return "", err
		}
		secs, err := strconv.ParseUint(args[3], 10, 32)
		if err != nil {
			return "", err
		}
		msg, err := stringArg(4)
		if err != nil {
			return "", err
		}
		return "", c.control.Mute(ctx, key, serverKey, peerKey, time.Duration(secs)*time.Second, msg)

	case "chat-get-mute":
		key, serverKey, err := networkServer(0)
		if err != nil {
			return "", err
		}
		res, err := c.control.GetMute(ctx, key, serverKey)
		if err != nil {
			return "", err
		}
		return fmt.Sprintf("%d %s", res.EndTime, hexs(res.Message)), nil

	case "chat-whisper":
		key, err := bytesArg(0)
		if err != nil {
			return "", err
		}
		peerKey, err := bytesArg(1)
		if err != nil {
			return "", err
		}
		alias, err := stringArg(2)
		if err != nil {
			return "", err
		}
		body, err := stringArg(3)
		if err != nil {
			return "", err
		}
		return chatWhisper(ctx, s, key, peerKey, alias, body)

	case "chat-whispers":
		peerKey, err := bytesArg(0)
		if err != nil {
			return "", err
		}
		thread, err := dao.ChatWhisperThreadsByPeerKey.Get(store, peerKey)
		if err != nil {
			return "", err
		}
		whispers, err := dao.ChatWhisperRecordsByPeerKey.GetAll(store, peerKey)
		if err != nil {
			return "", err
		}
		return messageHex(&chatv1.ListWhispersResponse{Thread: thread, Whispers: whispers})

	case "chat-whispers-read":
		peerKey, err := bytesArg(0)
		if err != nil {
			return "", err
		}
		return "", store.Update(func(tx kv.RWTx) error {
			id, err := dao.ChatWhisperThreadsByPeerKey.GetID(tx, peerKey)
			if err != nil {
				return err
			}
			_, err = dao.ChatWhisperThreads.Transform(tx, id, func(p *chatv1.WhisperThread) error {
				p.HasUnread = false
				return nil
			})
			if err != nil {
				return err
			}
			ids, err := dao.UnreadChatWhisperRecordsByPeerKey.GetAllIDs(tx, peerKey)
			if err != nil {
				return err
			}
			for _, id := range ids {
				_, err := dao.ChatWhisperRecords.Transform(tx, id, func(p *chatv1.WhisperRecord) error {
					p.State = chatv1.MessageState_MESSAGE_STATE_READ
					return nil
				})
				if err != nil {
					return err
				}
			}
			return nil
		})

	case "chat-ignore":
		peerKey, err := bytesArg(0)
		if err != nil {
			return "", err
		}
		alias, err := stringArg(1)
		if err != nil {
			return "", err
		}
		if err := need(3); err != nil {
			return "", err
		}
		var deadline int64
		if args[2] != "-" {
			secs, err := strconv.ParseInt(args[2], 10, 64)
			if err != nil {
				return "", err
			}
			deadline = timeutil.Now().Add(time.Duration(secs) * time.Second).Unix()
		}
		ignore, err := dao.NewChatUIConfigIgnore(store, alias, peerKey, deadline)
		if err != nil {
			return "", err
		}
		return "", store.Update(func(tx kv.RWTx) error {
			err := dao.ChatUIConfigIgnoresByPeerKey.Delete(tx, peerKey)
			if err != nil && !errors.Is(err, kv.ErrRecordNotFound) {
				return err
			}
			return dao.ChatUIConfigIgnores.Insert(tx, ignore)
		})

	case "chat-unignore":
		peerKey, err := bytesArg(0)
		if err != nil {
			return "", err
		}
		return "", dao.ChatUIConfigIgnoresByPeerKey.Delete(store, peerKey)

	case "chat-ignores":
		ms, err := dao.ChatUIConfigIgnores.GetAll(store)
		if err != nil {
			return "", err
		}
		return messageList(ms)
	}
	return "", fmt.Errorf("unknown command %q", verb)
}

// chatWhisper writes a whisper to the member with peerKey, or with alias
// when peerKey is nil, as internal/frontend's Whisper writes it: an
// ENQUEUED record, which the whisper runner delivers, and its thread.
func chatWhisper(ctx context.Context, s *networkState, key, peerKey []byte, alias, body string) (string, error) {
	cert, ok := s.control.Certificate(key)
	if !ok {
		return "", errors.New("no certificate found for network key")
	}

	var peerCert *certificate.Certificate
	var err error
	if peerKey != nil {
		peerCert, err = s.control.CA().FindByKey(ctx, key, peerKey)
	} else if alias != "" {
		peerCert, err = s.control.CA().FindBySubject(ctx, key, alias)
	} else {
		return "", errors.New("whisper subject undefined")
	}
	if err != nil {
		return "", fmt.Errorf("finding peer cert failed: %w", err)
	}

	record, err := dao.NewChatWhisperRecord(s.store, key, nil, peerCert.Key, cert, body, chat.ExtractMessageEntities(body), 0)
	if err != nil {
		return "", err
	}

	err = s.store.Update(func(tx kv.RWTx) error {
		thread, err := dao.ChatWhisperThreadsByPeerKey.Get(tx, peerCert.Key)
		if err != nil && !errors.Is(err, kv.ErrRecordNotFound) {
			return err
		}
		if thread == nil {
			thread, err = dao.NewChatWhisperThread(dao.ProfileID.IDGenerator(tx), peerCert)
			if err != nil {
				return err
			}
		}

		thread.Alias = peerCert.Subject
		thread.LastMessageTime = timeutil.Now().UnixNano() / int64(timeutil.Precision)
		thread.LastMessageId = record.Id

		if err := dao.ChatWhisperRecords.Insert(tx, record); err != nil {
			return err
		}
		return dao.ChatWhisperThreads.Upsert(tx, thread)
	})
	if err != nil {
		return "", err
	}
	return strconv.FormatUint(record.Id, 10), nil
}
