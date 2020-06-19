package dao

import (
	"strconv"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

const bootstrapClientPrefix = "bootstrapClient:"

func prefixBootstrapClientKey(id uint64) string {
	return bootstrapClientPrefix + strconv.FormatUint(id, 10)
}

// InsertBootstrapClient ...
func InsertBootstrapClient(s RWStore, v *pb.BootstrapClient) error {
	return s.Update(func(tx RWTx) (err error) {
		return tx.Put(prefixBootstrapClientKey(v.Id), v)
	})
}

// DeleteBootstrapClient ...
func DeleteBootstrapClient(s RWStore, id uint64) error {
	return s.Update(func(tx RWTx) (err error) {
		return tx.Delete(prefixBootstrapClientKey(id))
	})
}

// GetBootstrapClient ...
func GetBootstrapClient(s Store, id uint64) (v *pb.BootstrapClient, err error) {
	v = &pb.BootstrapClient{}
	err = s.View(func(tx Tx) error {
		return tx.Get(prefixBootstrapClientKey(id), v)
	})
	return
}

// GetBootstrapClients ...
func GetBootstrapClients(s Store) (v []*pb.BootstrapClient, err error) {
	v = []*pb.BootstrapClient{}
	err = s.View(func(tx Tx) error {
		return tx.ScanPrefix(bootstrapClientPrefix, &v)
	})
	return
}

// NewWebSocketBootstrapClient ...
func NewWebSocketBootstrapClient(url string) (*pb.BootstrapClient, error) {
	id, err := generateSnowflake()
	if err != nil {
		return nil, err
	}

	return &pb.BootstrapClient{
		Id: id,
		ClientOptions: &pb.BootstrapClient_WebsocketOptions{
			WebsocketOptions: &pb.BootstrapClientWebSocketOptions{
				Url: url,
			},
		},
	}, nil
}
