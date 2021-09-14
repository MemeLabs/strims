package dao

import (
	"strconv"

	chat "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
)

const (
	chatServerPrefix         = "chatServer:"
	chatEmotePrefix          = "chatEmote:"
	chatEmoteServerPrefix    = "chatEmoteServer:"
	chatModifierPrefix       = "chatModifier:"
	chatModifierServerPrefix = "chatModifierServer:"
	chatTagPrefix            = "chatTag:"
	chatTagServerPrefix      = "chatTagServer:"
	chatUIConfigKey          = "chatUIConfig"
)

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

	v := &chat.Server{
		Id:         id,
		NetworkKey: networkKey,
		Key:        key,
		Room:       chatRoom,
	}
	return v, nil
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
	effects []*chat.EmoteEffect,
	contributor *chat.EmoteContributor,
) (*chat.Emote, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	v := &chat.Emote{
		Id:          id,
		Name:        name,
		Images:      images,
		Effects:     effects,
		Contributor: contributor,
	}
	return v, nil
}

func prefixChatModifierKey(id uint64) string {
	return chatModifierPrefix + strconv.FormatUint(id, 10)
}

// InsertChatModifier ...
func InsertChatModifier(s kv.RWStore, serverID uint64, v *chat.Modifier) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		err = tx.Put(prefixChatModifierKey(v.Id), v)
		if err != nil {
			return err
		}
		return SetSecondaryIndex(tx, chatModifierServerPrefix, strconv.AppendUint(nil, serverID, 10), v.Id)
	})
}

// UpdateChatModifier ...
func UpdateChatModifier(s kv.RWStore, v *chat.Modifier) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		return tx.Put(prefixChatModifierKey(v.Id), v)
	})
}

// DeleteChatModifier ...
func DeleteChatModifier(s kv.RWStore, serverID, id uint64) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		err = DeleteSecondaryIndex(tx, chatModifierServerPrefix, strconv.AppendUint(nil, serverID, 10), id)
		if err != nil {
			return err
		}
		return tx.Delete(prefixChatModifierKey(id))
	})
}

// GetChatModifier ...
func GetChatModifier(s kv.Store, id uint64) (v *chat.Modifier, err error) {
	v = &chat.Modifier{}
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(prefixChatModifierKey(id), v)
	})
	return
}

// GetChatModifiers ...
func GetChatModifiers(s kv.Store, serverID uint64) (v []*chat.Modifier, err error) {
	v = []*chat.Modifier{}
	err = s.View(func(tx kv.Tx) error {
		ids, err := ScanSecondaryIndex(tx, chatModifierServerPrefix, strconv.AppendUint(nil, serverID, 10))
		if err != nil {
			return err
		}

		for _, id := range ids {
			e, err := GetChatModifier(tx, id)
			if err != nil {
				return err
			}
			v = append(v, e)
		}
		return nil
	})
	return
}

// NewChatModifier ...
func NewChatModifier(
	g IDGenerator,
	name string,
	priority uint32,
	internal bool,
) (*chat.Modifier, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	v := &chat.Modifier{
		Id:       id,
		Name:     name,
		Priority: priority,
		Internal: internal,
	}
	return v, nil
}

func prefixChatTagKey(id uint64) string {
	return chatTagPrefix + strconv.FormatUint(id, 10)
}

// InsertChatTag ...
func InsertChatTag(s kv.RWStore, serverID uint64, v *chat.Tag) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		err = tx.Put(prefixChatTagKey(v.Id), v)
		if err != nil {
			return err
		}
		return SetSecondaryIndex(tx, chatTagServerPrefix, strconv.AppendUint(nil, serverID, 10), v.Id)
	})
}

// UpdateChatTag ...
func UpdateChatTag(s kv.RWStore, v *chat.Tag) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		return tx.Put(prefixChatTagKey(v.Id), v)
	})
}

// DeleteChatTag ...
func DeleteChatTag(s kv.RWStore, serverID, id uint64) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		err = DeleteSecondaryIndex(tx, chatTagServerPrefix, strconv.AppendUint(nil, serverID, 10), id)
		if err != nil {
			return err
		}
		return tx.Delete(prefixChatTagKey(id))
	})
}

// GetChatTag ...
func GetChatTag(s kv.Store, id uint64) (v *chat.Tag, err error) {
	v = &chat.Tag{}
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(prefixChatTagKey(id), v)
	})
	return
}

// GetChatTags ...
func GetChatTags(s kv.Store, serverID uint64) (v []*chat.Tag, err error) {
	v = []*chat.Tag{}
	err = s.View(func(tx kv.Tx) error {
		ids, err := ScanSecondaryIndex(tx, chatTagServerPrefix, strconv.AppendUint(nil, serverID, 10))
		if err != nil {
			return err
		}

		for _, id := range ids {
			e, err := GetChatTag(tx, id)
			if err != nil {
				return err
			}
			v = append(v, e)
		}
		return nil
	})
	return
}

// NewChatTag ...
func NewChatTag(
	g IDGenerator,
	name string,
	color string,
	sensitive bool,
) (*chat.Tag, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	v := &chat.Tag{
		Id:        id,
		Name:      name,
		Color:     color,
		Sensitive: sensitive,
	}
	return v, nil
}

// SetChatUIConfig ...
func SetChatUIConfig(s kv.RWStore, v *chat.UIConfig) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		return tx.Put(chatUIConfigKey, v)
	})
}

// GetChatUIConfig ...
func GetChatUIConfig(s kv.Store) (v *chat.UIConfig, err error) {
	v = &chat.UIConfig{}
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(chatUIConfigKey, v)
	})
	return
}
