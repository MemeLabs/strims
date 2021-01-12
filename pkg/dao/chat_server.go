package dao

import (
	"strconv"

	chat "github.com/MemeLabs/go-ppspp/pkg/apis/chat/v1"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
)

const chatServerPrefix = "chatServer:"

func prefixChatServerKey(id uint64) string {
	return chatServerPrefix + strconv.FormatUint(id, 10)
}

// InsertChatServer ...
func InsertChatServer(s kv.RWStore, v *chat.ChatServer) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		return tx.Put(prefixChatServerKey(v.Id), v)
	})
}

// DeleteChatServer ...
func DeleteChatServer(s kv.RWStore, id uint64) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		return tx.Delete(prefixChatServerKey(id))
	})
}

// GetChatServer ...
func GetChatServer(s kv.Store, id uint64) (v *chat.ChatServer, err error) {
	v = &chat.ChatServer{}
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(prefixChatServerKey(id), v)
	})
	return
}

// GetChatServers ...
func GetChatServers(s kv.Store) (v []*chat.ChatServer, err error) {
	v = []*chat.ChatServer{}
	err = s.View(func(tx kv.Tx) error {
		return tx.ScanPrefix(chatServerPrefix, &v)
	})
	return
}

// NewChatServer ...
func NewChatServer(g IDGenerator, networkKey []byte, chatRoom *chat.ChatRoom) (*chat.ChatServer, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	key, err := GenerateKey()
	if err != nil {
		return nil, err
	}

	network := &chat.ChatServer{
		Id:         id,
		NetworkKey: networkKey,
		Key:        key,
		ChatRoom:   chatRoom,
	}
	return network, nil
}
