package dao

import (
	"strconv"

	chat "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
)

const chatServerPrefix = "chatServer:"
const chatEmotePrefix = "chatEmote:"
const chatEmoteServerPrefix = "chatEmoteServer:"

func prefixChatServerKey(id uint64) string {
	return chatServerPrefix + strconv.FormatUint(id, 10)
}

// UpsertChatServer ...
func UpsertChatServer(s kv.RWStore, v *chat.Server) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		return tx.Put(prefixChatServerKey(v.Id), v)
	})
}

// DeleteChatServer ...
func DeleteChatServer(s kv.RWStore, id uint64) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		emoteIDs, err := ScanSecondaryIndex(tx, chatEmoteServerPrefix, strconv.AppendUint(nil, id, 10))
		if err != nil {
			return err
		}
		for _, emoteID := range emoteIDs {
			if err := DeleteChatEmote(tx, id, emoteID); err != nil {
				return err
			}
		}

		return tx.Delete(prefixChatServerKey(id))
	})
}

// GetChatServer ...
func GetChatServer(s kv.Store, id uint64) (v *chat.Server, err error) {
	v = &chat.Server{}
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(prefixChatServerKey(id), v)
	})
	return
}

// GetChatServers ...
func GetChatServers(s kv.Store) (v []*chat.Server, err error) {
	v = []*chat.Server{}
	err = s.View(func(tx kv.Tx) error {
		return tx.ScanPrefix(chatServerPrefix, &v)
	})
	return
}

// NewChatServer ...
func NewChatServer(g IDGenerator, networkKey []byte, chatRoom *chat.Room) (*chat.Server, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	key, err := GenerateKey()
	if err != nil {
		return nil, err
	}

	network := &chat.Server{
		Id:         id,
		NetworkKey: networkKey,
		Key:        key,
		Room:       chatRoom,
	}
	return network, nil
}

func prefixChatEmoteKey(id uint64) string {
	return chatEmotePrefix + strconv.FormatUint(id, 10)
}

// InsertChatEmote ...
func InsertChatEmote(s kv.RWStore, serverID uint64, v *chat.Emote) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		err = tx.Put(prefixChatEmoteKey(v.Id), v)
		if err != nil {
			return err
		}
		return SetSecondaryIndex(tx, chatEmoteServerPrefix, strconv.AppendUint(nil, serverID, 10), v.Id)
	})
}

// UpdateChatEmote ...
func UpdateChatEmote(s kv.RWStore, v *chat.Emote) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		return tx.Put(prefixChatEmoteKey(v.Id), v)
	})
}

// DeleteChatEmote ...
func DeleteChatEmote(s kv.RWStore, serverID, id uint64) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		err = DeleteSecondaryIndex(tx, chatEmoteServerPrefix, strconv.AppendUint(nil, serverID, 10), id)
		if err != nil {
			return err
		}
		return tx.Delete(prefixChatEmoteKey(id))
	})
}

// GetChatEmote ...
func GetChatEmote(s kv.Store, id uint64) (v *chat.Emote, err error) {
	v = &chat.Emote{}
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(prefixChatEmoteKey(id), v)
	})
	return
}

// GetChatEmotes ...
func GetChatEmotes(s kv.Store, serverID uint64) (v []*chat.Emote, err error) {
	v = []*chat.Emote{}
	err = s.View(func(tx kv.Tx) error {
		ids, err := ScanSecondaryIndex(tx, chatEmoteServerPrefix, strconv.AppendUint(nil, serverID, 10))
		if err != nil {
			return err
		}

		for _, id := range ids {
			e, err := GetChatEmote(tx, id)
			if err != nil {
				return err
			}
			v = append(v, e)
		}
		return nil
	})
	return
}

// NewChatEmote ...
func NewChatEmote(
	g IDGenerator,
	name string,
	images []*chat.EmoteImage,
	css string,
	animation *chat.EmoteAnimation,
) (*chat.Emote, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	network := &chat.Emote{
		Id:        id,
		Name:      name,
		Images:    images,
		Css:       css,
		Animation: animation,
	}
	return network, nil
}
