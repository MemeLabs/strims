package dao

import (
	"bytes"
	"encoding/binary"

	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/hashmap"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
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

var GetChatEmotesByServerID, GetChatEmotesByServer, GetChatServerByEmote = ManyToOne(
	chatEmoteServerNS,
	ChatEmotes,
	ChatServers,
	(*chatv1.Emote).GetServerId,
	&ManyToOneOptions{CascadeDelete: true},
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

var GetChatModifiersByServerID, GetChatModifiersByServer, GetChatServerByModifier = ManyToOne(
	chatModifierServerNS,
	ChatModifiers,
	ChatServers,
	(*chatv1.Modifier).GetServerId,
	&ManyToOneOptions{CascadeDelete: true},
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

var GetChatTagsByServerID, GetChatTagsByServer, GetChatServerByTag = ManyToOne(
	chatTagServerNS,
	ChatTags,
	ChatServers,
	(*chatv1.Tag).GetServerId,
	&ManyToOneOptions{CascadeDelete: true},
)

var ChatProfiles = NewTable[chatv1.Profile](chatProfileNS, nil)

var GetChatProfilesByServerID, GetChatProfilesByServer, GetChatServerByProfile = ManyToOne(
	chatProfileServerNS,
	ChatProfiles,
	ChatServers,
	(*chatv1.Profile).GetServerId,
	&ManyToOneOptions{CascadeDelete: true},
)

func FormatChatProfilePeerKey(serverID uint64, peerKey []byte) []byte {
	b := make([]byte, 8, 8+len([]byte(peerKey)))
	binary.BigEndian.PutUint64(b, serverID)
	return append(b, peerKey...)
}

func chatProfilePeerKey(m *chatv1.Profile) []byte {
	return FormatChatProfilePeerKey(m.ServerId, m.PeerKey)
}

var GetChatProfileByPeerKey = UniqueIndex(chatProfilePeerKeyNS, ChatProfiles, chatProfilePeerKey, nil)

func NewChatProfileCache(s kv.RWStore, opt *CacheStoreOptions) (c ChatProfileCache) {
	c.CacheStore, c.ByID = newCacheStore[chatv1.Profile](s, ChatProfiles, opt)
	c.ByPeerKey = NewCacheIndex(
		c.CacheStore,
		GetChatProfileByPeerKey,
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

var ChatWhisperThreads = NewTable(
	chatWhisperThreadNS,
	&TableOptions[chatv1.WhisperThread, *chatv1.WhisperThread]{
		ObserveChange: func(m, p *chatv1.WhisperThread) proto.Message {
			return &chatv1.WhisperThreadChangeEvent{WhisperThread: m}
		},
	},
)

var GetChatWhisperThreadByPeerKey = UniqueIndex(
	chatWhisperThreadPeerKeyNS,
	ChatWhisperThreads,
	(*chatv1.WhisperThread).GetPeerKey,
	nil,
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

var GetChatWhisperRecordsByPeerKey = SecondaryIndex(
	chatWhisperRecordPeerKeyNS,
	ChatWhisperRecords,
	(*chatv1.WhisperRecord).GetPeerKey,
)

func FormatChatWhisperRecordStateKey(s chatv1.WhisperRecord_State) []byte {
	b := make([]byte, 4)
	binary.BigEndian.PutUint32(b, uint32(s))
	return b
}

var GetChatWhisperRecordsByState = SecondaryIndex(
	chatWhisperRecordStateNS,
	ChatWhisperRecords,
	func(m *chatv1.WhisperRecord) []byte { return FormatChatWhisperRecordStateKey(m.State) },
)

// NewChatServer ...
func NewChatServer(g IDGenerator, networkKey []byte, chatRoom *chatv1.Room) (*chatv1.Server, error) {
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
) (*chatv1.Modifier, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	v := &chatv1.Modifier{
		Id:       id,
		ServerId: serverID,
		Name:     name,
		Priority: priority,
		Internal: internal,
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
) (*chatv1.WhisperRecord, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	state := chatv1.WhisperRecord_WHISPER_STATE_ENQUEUED
	if bytes.Equal(peerKey, cert.Key) {
		state = chatv1.WhisperRecord_WHISPER_STATE_RECEIVED
	}

	return &chatv1.WhisperRecord{
		Id:         id,
		NetworkKey: networkKey,
		ServerKey:  serverKey,
		PeerKey:    peerKey,
		State:      state,
		Message: &chatv1.Message{
			ServerTime: timeutil.Now().UnixNano() / int64(timeutil.Precision),
			PeerKey:    cert.Key,
			Nick:       cert.Subject,
			Body:       body,
			Entities:   &chatv1.Message_Entities{},
		},
	}, nil
}
