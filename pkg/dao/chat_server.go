package dao

import (
	"strconv"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

const chatServerPrefix = "chatServer:"

func prefixChatServerKey(id uint64) string {
	return chatServerPrefix + strconv.FormatUint(id, 10)
}

// InsertChatServer ...
func InsertChatServer(s RWStore, v *pb.ChatServer) error {
	return s.Update(func(tx RWTx) (err error) {
		return tx.Put(prefixChatServerKey(v.Id), v)
	})
}

// DeleteChatServer ...
func DeleteChatServer(s RWStore, id uint64) error {
	return s.Update(func(tx RWTx) (err error) {
		return tx.Delete(prefixChatServerKey(id))
	})
}

// GetChatServer ...
func GetChatServer(s Store, id uint64) (v *pb.ChatServer, err error) {
	v = &pb.ChatServer{}
	err = s.View(func(tx Tx) error {
		return tx.Get(prefixChatServerKey(id), v)
	})
	return
}

// GetChatServers ...
func GetChatServers(s Store) (v []*pb.ChatServer, err error) {
	v = []*pb.ChatServer{}
	err = s.View(func(tx Tx) error {
		return tx.ScanPrefix(chatServerPrefix, &v)
	})
	return
}

// NewChatServer ...
func NewChatServer(networkKey []byte, chatRoom *pb.ChatRoom) (*pb.ChatServer, error) {
	id, err := generateSnowflake()
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
