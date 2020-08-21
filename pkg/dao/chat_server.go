package dao

import (
	"strconv"

	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

const chatServerPrefix = "chatServer:"

func prefixChatServerKey(id uint64) string {
	return chatServerPrefix + strconv.FormatUint(id, 10)
}

// InsertChatServer ...
func InsertChatServer(s kv.RWStore, v *pb.ChatServer) error {
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
func GetChatServer(s kv.Store, id uint64) (v *pb.ChatServer, err error) {
	v = &pb.ChatServer{}
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(prefixChatServerKey(id), v)
	})
	return
}

// GetChatServers ...
func GetChatServers(s kv.Store) (v []*pb.ChatServer, err error) {
	v = []*pb.ChatServer{}
	err = s.View(func(tx kv.Tx) error {
		return tx.ScanPrefix(chatServerPrefix, &v)
	})
	return
}

// NewChatServer ...
func NewChatServer(networkKey []byte, chatRoom *pb.ChatRoom) (*pb.ChatServer, error) {
	id, err := GenerateSnowflake()
	if err != nil {
		return nil, err
	}

	key, err := GenerateKey()
	if err != nil {
		return nil, err
	}

	network := &pb.ChatServer{
		Id:         id,
		NetworkKey: networkKey,
		Key:        key,
		ChatRoom:   chatRoom,
	}
	return network, nil
}
