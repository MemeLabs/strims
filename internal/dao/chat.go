package dao

import (
	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
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
)

var ChatServers = NewTable(
	chatServerNS,
	&TableOptions[chatv1.Server]{
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
	&TableOptions[chatv1.Emote]{
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
	&TableOptions[chatv1.Modifier]{
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
	&TableOptions[chatv1.Tag]{
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

var ChatUIConfig = NewSingleton(
	chatUIConfigNS,
	&SingletonOptions[chatv1.UIConfig]{
		ObserveChange: func(m, p *chatv1.UIConfig) proto.Message {
			return &chatv1.UIConfigChangeEvent{UiConfig: m}
		},
	},
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
