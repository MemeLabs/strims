package dao

import (
	chatv1 "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
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

var ChatServers = NewTable[chatv1.Server](chatServerNS)

var ChatEmotes = NewTable[chatv1.Emote](chatEmoteNS)

var GetChatEmotesByServerID, GetChatEmotesByServer, GetChatServerByEmote = ManyToOne(
	chatEmoteServerNS,
	ChatEmotes,
	ChatServers,
	(*chatv1.Emote).GetServerId,
	&ManyToOneOptions{CascadeDelete: true},
)

var ChatModifiers = NewTable[chatv1.Modifier](chatModifierNS)

var GetChatModifiersByServerID, GetChatModifiersByServer, GetChatServerByModifier = ManyToOne(
	chatModifierServerNS,
	ChatModifiers,
	ChatServers,
	(*chatv1.Modifier).GetServerId,
	&ManyToOneOptions{CascadeDelete: true},
)

var ChatTags = NewTable[chatv1.Tag](chatTagNS)

var GetChatTagsByServerID, GetChatTagsByServer, GetChatServerByTag = ManyToOne(
	chatTagServerNS,
	ChatTags,
	ChatServers,
	(*chatv1.Tag).GetServerId,
	&ManyToOneOptions{CascadeDelete: true},
)

var ChatUIConfig = NewSingleton[chatv1.UIConfig](chatUIConfigNS, nil)

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
