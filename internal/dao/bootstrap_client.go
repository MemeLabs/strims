package dao

import (
	"strconv"

	"github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/bootstrap"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
)

const bootstrapClientPrefix = "bootstrapClient:"

func prefixBootstrapClientKey(id uint64) string {
	return bootstrapClientPrefix + strconv.FormatUint(id, 10)
}

// InsertBootstrapClient ...
func InsertBootstrapClient(s kv.RWStore, v *bootstrap.BootstrapClient) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		return tx.Put(prefixBootstrapClientKey(v.Id), v)
	})
}

// DeleteBootstrapClient ...
func DeleteBootstrapClient(s kv.RWStore, id uint64) error {
	return s.Update(func(tx kv.RWTx) (err error) {
		return tx.Delete(prefixBootstrapClientKey(id))
	})
}

// GetBootstrapClient ...
func GetBootstrapClient(s kv.Store, id uint64) (v *bootstrap.BootstrapClient, err error) {
	v = &bootstrap.BootstrapClient{}
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(prefixBootstrapClientKey(id), v)
	})
	return
}

// GetBootstrapClients ...
func GetBootstrapClients(s kv.Store) (v []*bootstrap.BootstrapClient, err error) {
	v = []*bootstrap.BootstrapClient{}
	err = s.View(func(tx kv.Tx) error {
		return tx.ScanPrefix(bootstrapClientPrefix, &v)
	})
	return
}

// NewWebSocketBootstrapClient ...
func NewWebSocketBootstrapClient(g IDGenerator, url string, insecureSkipVerifyTLS bool) (*bootstrap.BootstrapClient, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	return &bootstrap.BootstrapClient{
		Id: id,
		ClientOptions: &bootstrap.BootstrapClient_WebsocketOptions{
			WebsocketOptions: &bootstrap.BootstrapClientWebSocketOptions{
				Url:                   url,
				InsecureSkipVerifyTls: insecureSkipVerifyTLS,
			},
		},
	}, nil
}
