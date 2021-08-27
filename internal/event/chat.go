package event

import "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"

// ChatServerSync ...
type ChatServerSync struct {
	Server *chat.Server
}

// ChatServerRemove ...
type ChatServerRemove struct {
	ID uint64
}

// ChatEmoteSync ...
type ChatEmoteSync struct {
	ServerID uint64
	Emote    *chat.Emote
}

// ChatEmoteRemove ...
type ChatEmoteRemove struct {
	ID uint64
}

// ChatSyncAssets ...
type ChatSyncAssets struct {
	ServerID           uint64
	ForceUnifiedUpdate bool
}
