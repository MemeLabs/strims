// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package dao

import (
	"bytes"
	"encoding/binary"

	"github.com/MemeLabs/strims/internal/dao/versionvector"
	chatv1 "github.com/MemeLabs/strims/pkg/apis/chat/v1"
	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
	"github.com/MemeLabs/strims/pkg/hashmap"
	"github.com/MemeLabs/strims/pkg/kv"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"google.golang.org/protobuf/proto"
)

const (
	_ = iota + chatNS
	chatServerNS
	chatEmoteNS
	chatEmoteServerNS
	chatModifierNS
	chatModifierServerNS
	chatTagNS
	chatTagServerNS
	chatUIConfigNS
	chatProfileNS
	chatProfileServerNS
	chatProfilePeerKeyNS
	chatWhisperRecordNS
	chatWhisperRecordPeerKeyNS
	chatWhisperRecordStateNS
	chatWhisperThreadNS
	chatWhisperThreadPeerKeyNS
	chatWhisperRecordWhisperUnreadNS
	chatUIConfigHighlightNS
	chatUIConfigHighlightKeyNS
	chatUIConfigTagNS
	chatUIConfigTagKeyNS
	chatUIConfigIgnoreNS
	chatUIConfigIgnoreKeyNS
	chatServerIconNS
)

var ChatServers = NewTable(
	chatServerNS,
	&TableOptions[chatv1.Server, *chatv1.Server]{
		ObserveChange: func(m, p *chatv1.Server) proto.Message {
			return &chatv1.ServerChangeEvent{Server: m}
		},
		ObserveDelete: func(m *chatv1.Server) proto.Message {
			return &chatv1.ServerDeleteEvent{Server: m}
		},
	},
)

var ChatServerIcons = NewTable(
	chatServerIconNS,
	&TableOptions[chatv1.ServerIcon, *chatv1.ServerIcon]{
		ObserveChange: func(m, p *chatv1.ServerIcon) proto.Message {
			return &chatv1.ServerIconChangeEvent{ServerIcon: m}
		},
	},
)

var ChatEmotes = NewTable(
	chatEmoteNS,
	&TableOptions[chatv1.Emote, *chatv1.Emote]{
		ObserveChange: func(m, p *chatv1.Emote) proto.Message {
			return &chatv1.EmoteChangeEvent{Emote: m}
		},
		ObserveDelete: func(m *chatv1.Emote) proto.Message {
			return &chatv1.EmoteDeleteEvent{Emote: m}
		},
	},
)

var ChatEmotesByServer = ManyToOne(
	chatEmoteServerNS,
	ChatEmotes,
	ChatServers,
	(*chatv1.Emote).GetServerId,
	&ManyToOneOptions[chatv1.Emote, *chatv1.Emote]{CascadeDelete: true},
)

var ChatModifiers = NewTable(
	chatModifierNS,
	&TableOptions[chatv1.Modifier, *chatv1.Modifier]{
		ObserveChange: func(m, p *chatv1.Modifier) proto.Message {
			return &chatv1.ModifierChangeEvent{Modifier: m}
		},
		ObserveDelete: func(m *chatv1.Modifier) proto.Message {
			return &chatv1.ModifierDeleteEvent{Modifier: m}
		},
	},
)

var ChatModifiersByServer = ManyToOne(
	chatModifierServerNS,
	ChatModifiers,
	ChatServers,
	(*chatv1.Modifier).GetServerId,
	&ManyToOneOptions[chatv1.Modifier, *chatv1.Modifier]{CascadeDelete: true},
)

var ChatTags = NewTable(
	chatTagNS,
	&TableOptions[chatv1.Tag, *chatv1.Tag]{
		ObserveChange: func(m, p *chatv1.Tag) proto.Message {
			return &chatv1.TagChangeEvent{Tag: m}
		},
		ObserveDelete: func(m *chatv1.Tag) proto.Message {
			return &chatv1.TagDeleteEvent{Tag: m}
		},
	},
)

var ChatTagsByServer = ManyToOne(
	chatTagServerNS,
	ChatTags,
	ChatServers,
	(*chatv1.Tag).GetServerId,
	&ManyToOneOptions[chatv1.Tag, *chatv1.Tag]{CascadeDelete: true},
)

var ChatProfiles = NewTable[chatv1.Profile](chatProfileNS, nil)

var ChatProfilesByServer = ManyToOne(
	chatProfileServerNS,
	ChatProfiles,
	ChatServers,
	(*chatv1.Profile).GetServerId,
	&ManyToOneOptions[chatv1.Profile, *chatv1.Profile]{CascadeDelete: true},
)

func FormatChatProfilePeerKey(serverID uint64, peerKey []byte) []byte {
	b := make([]byte, 8, 8+len([]byte(peerKey)))
	binary.BigEndian.PutUint64(b, serverID)
	return append(b, peerKey...)
}

func chatProfilePeerKey(m *chatv1.Profile) []byte {
	return FormatChatProfilePeerKey(m.ServerId, m.PeerKey)
}

var ChatProfilesByPeerKey = NewUniqueIndex(chatProfilePeerKeyNS, ChatProfiles, chatProfilePeerKey, byteIdentity, nil)

func NewChatProfileCache(s kv.RWStore, opt *CacheStoreOptions) (c ChatProfileCache) {
	c.CacheStore, c.ByID = newCacheStore[chatv1.Profile](s, ChatProfiles, opt)
	c.ByPeerKey = NewCacheIndex(
		c.CacheStore,
		ChatProfilesByPeerKey.Get,
		chatProfilePeerKey,
		hashmap.NewByteInterface[[]byte],
	)
	return
}

type ChatProfileCache struct {
	*CacheStore[chatv1.Profile, *chatv1.Profile]
	ByID      CacheAccessor[uint64, chatv1.Profile, *chatv1.Profile]
	ByPeerKey CacheAccessor[[]byte, chatv1.Profile, *chatv1.Profile]
}

var ChatUIConfig = NewSingleton(
	chatUIConfigNS,
	&SingletonOptions[chatv1.UIConfig, *chatv1.UIConfig]{
		ObserveChange: func(m, p *chatv1.UIConfig) proto.Message {
			return &chatv1.UIConfigChangeEvent{UiConfig: m}
		},
	},
)

var ChatUIConfigHighlights = NewTable(
	chatUIConfigHighlightNS,
	&TableOptions[chatv1.UIConfigHighlight, *chatv1.UIConfigHighlight]{
		ObserveChange: func(m, p *chatv1.UIConfigHighlight) proto.Message {
			return &chatv1.UIConfigHighlightChangeEvent{UiConfigHighlight: m}
		},
		ObserveDelete: func(m *chatv1.UIConfigHighlight) proto.Message {
			return &chatv1.UIConfigHighlightDeleteEvent{UiConfigHighlight: m}
		},
	},
)

func init() {
	RegisterReplicatedTable(ChatUIConfigHighlights, nil)
}

var ChatUIConfigHighlightsByPeerKey = NewUniqueIndex(chatUIConfigHighlightKeyNS, ChatUIConfigHighlights, (*chatv1.UIConfigHighlight).GetPeerKey, byteIdentity, nil)

var ChatUIConfigTags = NewTable(
	chatUIConfigTagNS,
	&TableOptions[chatv1.UIConfigTag, *chatv1.UIConfigTag]{
		ObserveChange: func(m, p *chatv1.UIConfigTag) proto.Message {
			return &chatv1.UIConfigTagChangeEvent{UiConfigTag: m}
		},
		ObserveDelete: func(m *chatv1.UIConfigTag) proto.Message {
			return &chatv1.UIConfigTagDeleteEvent{UiConfigTag: m}
		},
	},
)

func init() {
	RegisterReplicatedTable(ChatUIConfigTags, nil)
}

var ChatUIConfigTagsByPeerKey = NewUniqueIndex(chatUIConfigTagKeyNS, ChatUIConfigTags, (*chatv1.UIConfigTag).GetPeerKey, byteIdentity, nil)

var ChatUIConfigIgnores = NewTable(
	chatUIConfigIgnoreNS,
	&TableOptions[chatv1.UIConfigIgnore, *chatv1.UIConfigIgnore]{
		ObserveChange: func(m, p *chatv1.UIConfigIgnore) proto.Message {
			return &chatv1.UIConfigIgnoreChangeEvent{UiConfigIgnore: m}
		},
		ObserveDelete: func(m *chatv1.UIConfigIgnore) proto.Message {
			return &chatv1.UIConfigIgnoreDeleteEvent{UiConfigIgnore: m}
		},
	},
)

func init() {
	RegisterReplicatedTable(ChatUIConfigIgnores, nil)
}

var ChatUIConfigIgnoresByPeerKey = NewUniqueIndex(chatUIConfigIgnoreKeyNS, ChatUIConfigIgnores, (*chatv1.UIConfigIgnore).GetPeerKey, byteIdentity, nil)

var ChatWhisperThreads = NewTable(
	chatWhisperThreadNS,
	&TableOptions[chatv1.WhisperThread, *chatv1.WhisperThread]{
		ObserveChange: func(m, p *chatv1.WhisperThread) proto.Message {
			return &chatv1.WhisperThreadChangeEvent{WhisperThread: m}
		},
		ObserveDelete: func(m *chatv1.WhisperThread) proto.Message {
			return &chatv1.WhisperThreadDeleteEvent{WhisperThread: m}
		},
		OnDelete: LocalDeleteHook(func(s ReplicatedRWTx, p *chatv1.WhisperThread) error {
			_, err := ChatWhisperRecordsByPeerKey.DeleteAll(s, p.PeerKey)
			return err
		}),
	},
)

func resolveChatWhisperThreadConflict(m, p *chatv1.WhisperThread) {
	versionvector.Upgrade(m.GetVersion(), p.GetVersion())
	if p.LastMessageTime > m.LastMessageTime {
		m.Alias = p.Alias
		m.LastMessageTime = p.LastMessageTime
		m.LastMessageId = p.LastMessageId
	}
	m.HasUnread = m.HasUnread || p.HasUnread
}

func init() {
	RegisterReplicatedTable(
		ChatWhisperThreads,
		&ReplicatedTableOptions[*chatv1.WhisperThread]{
			OnConflict: func(s kv.RWStore, m *chatv1.WhisperThread, p *chatv1.WhisperThread) error {
				resolveChatWhisperThreadConflict(m, p)
				return ChatWhisperThreads.Update(s, m)
			},
		},
	)
}

var ChatWhisperThreadsByPeerKey = NewUniqueIndex(
	chatWhisperThreadPeerKeyNS,
	ChatWhisperThreads,
	(*chatv1.WhisperThread).GetPeerKey,
	byteIdentity,
	&UniqueIndexOptions[chatv1.WhisperThread, *chatv1.WhisperThread]{
		OnConflict: func(s kv.RWStore, t *Table[chatv1.WhisperThread, *chatv1.WhisperThread], m, p *chatv1.WhisperThread) error {
			resolveChatWhisperThreadConflict(m, p)
			return ChatWhisperThreads.Delete(s, p.Id)
		},
	},
)

var ChatWhisperRecords = NewTable(
	chatWhisperRecordNS,
	&TableOptions[chatv1.WhisperRecord, *chatv1.WhisperRecord]{
		ObserveChange: func(m, p *chatv1.WhisperRecord) proto.Message {
			return &chatv1.WhisperRecordChangeEvent{WhisperRecord: m}
		},
		ObserveDelete: func(m *chatv1.WhisperRecord) proto.Message {
			return &chatv1.WhisperRecordDeleteEvent{WhisperRecord: m}
		},
	},
)

func init() {
	RegisterReplicatedTable(ChatWhisperRecords, nil)
}

var ChatWhisperRecordsByPeerKey = NewSecondaryIndex(
	chatWhisperRecordPeerKeyNS,
	ChatWhisperRecords,
	(*chatv1.WhisperRecord).GetPeerKey,
	byteIdentity,
	nil,
)

var UnreadChatWhisperRecordsByPeerKey = NewSecondaryIndex(
	chatWhisperRecordWhisperUnreadNS,
	ChatWhisperRecords,
	(*chatv1.WhisperRecord).GetPeerKey,
	byteIdentity,
	&SecondaryIndexOptions[chatv1.WhisperRecord, *chatv1.WhisperRecord]{
		Condition: func(m *chatv1.WhisperRecord) bool {
			return m.State == chatv1.WhisperRecord_WHISPER_STATE_UNREAD
		},
	},
)

func FormatChatWhisperRecordStateKey(s chatv1.WhisperRecord_State) []byte {
	b := make([]byte, 4)
	binary.BigEndian.PutUint32(b, uint32(s))
	return b
}

var ChatWhisperRecordsByState = NewSecondaryIndex(
	chatWhisperRecordStateNS,
	ChatWhisperRecords,
	func(m *chatv1.WhisperRecord) []byte { return FormatChatWhisperRecordStateKey(m.State) },
	byteIdentity,
	nil,
)

// NewChatServer ...
func NewChatServer(
	g IDGenerator,
	networkKey []byte,
	chatRoom *chatv1.Room,
) (*chatv1.Server, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	key, err := GenerateKey()
	if err != nil {
		return nil, err
	}

	v := &chatv1.Server{
		Id:         id,
		NetworkKey: networkKey,
		Key:        key,
		Room:       chatRoom,
	}
	return v, nil
}

// NewChatEmote ...
func NewChatEmote(
	g IDGenerator,
	serverID uint64,
	name string,
	images []*chatv1.EmoteImage,
	effects []*chatv1.EmoteEffect,
	contributor *chatv1.EmoteContributor,
) (*chatv1.Emote, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	v := &chatv1.Emote{
		Id:          id,
		ServerId:    serverID,
		Name:        name,
		Images:      images,
		Effects:     effects,
		Contributor: contributor,
	}
	return v, nil
}

// NewChatModifier ...
func NewChatModifier(
	g IDGenerator,
	serverID uint64,
	name string,
	priority uint32,
	internal bool,
	extraWrapCount uint32,
	procChance float64,
) (*chatv1.Modifier, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	v := &chatv1.Modifier{
		Id:             id,
		ServerId:       serverID,
		Name:           name,
		Priority:       priority,
		Internal:       internal,
		ExtraWrapCount: extraWrapCount,
		ProcChance:     procChance,
	}
	return v, nil
}

// NewChatTag ...
func NewChatTag(
	g IDGenerator,
	serverID uint64,
	name string,
	color string,
	sensitive bool,
) (*chatv1.Tag, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	v := &chatv1.Tag{
		Id:        id,
		ServerId:  serverID,
		Name:      name,
		Color:     color,
		Sensitive: sensitive,
	}
	return v, nil
}

// NewChatProfile ...
func NewChatProfile(
	g IDGenerator,
	serverID uint64,
	peerKey []byte,
	alias string,
) (*chatv1.Profile, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	v := &chatv1.Profile{
		Id:       id,
		ServerId: serverID,
		PeerKey:  peerKey,
		Alias:    alias,
	}
	return v, nil
}

// NewChatWhisperThread ...
func NewChatWhisperThread(
	g IDGenerator,
	peerCert *certificate.Certificate,
) (*chatv1.WhisperThread, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	v := &chatv1.WhisperThread{
		Id:      id,
		Version: versionvector.New(),
		PeerKey: peerCert.Key,
		Alias:   peerCert.Subject,
	}
	return v, nil
}

// NewChatWhisperRecord ...
func NewChatWhisperRecord(
	g IDGenerator,
	networkKey []byte,
	serverKey []byte,
	peerKey []byte,
	cert *certificate.Certificate,
	body string,
	entities *chatv1.Message_Entities,
) (*chatv1.WhisperRecord, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	state := chatv1.WhisperRecord_WHISPER_STATE_ENQUEUED
	if bytes.Equal(peerKey, cert.Key) {
		state = chatv1.WhisperRecord_WHISPER_STATE_UNREAD
	}

	return &chatv1.WhisperRecord{
		Id:         id,
		Version:    versionvector.New(),
		NetworkKey: networkKey,
		ServerKey:  serverKey,
		PeerKey:    peerKey,
		State:      state,
		Message: &chatv1.Message{
			ServerTime: timeutil.Now().UnixNano() / int64(timeutil.Precision),
			PeerKey:    cert.Key,
			Nick:       cert.Subject,
			Body:       body,
			Entities:   entities,
		},
	}, nil
}

func NewChatUIConfigHighlight(
	g IDGenerator,
	alias string,
	peerKey []byte,
) (*chatv1.UIConfigHighlight, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	v := &chatv1.UIConfigHighlight{
		Id:      id,
		Version: versionvector.New(),
		Alias:   alias,
		PeerKey: peerKey,
	}
	return v, nil
}

func NewChatUIConfigTag(
	g IDGenerator,
	alias string,
	peerKey []byte,
	color string,
) (*chatv1.UIConfigTag, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	v := &chatv1.UIConfigTag{
		Id:      id,
		Version: versionvector.New(),
		Alias:   alias,
		PeerKey: peerKey,
		Color:   color,
	}
	return v, nil
}

func NewChatUIConfigIgnore(
	g IDGenerator,
	alias string,
	peerKey []byte,
	deadline int64,
) (*chatv1.UIConfigIgnore, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	v := &chatv1.UIConfigIgnore{
		Id:       id,
		Version:  versionvector.New(),
		Alias:    alias,
		PeerKey:  peerKey,
		Deadline: deadline,
	}
	return v, nil
}
