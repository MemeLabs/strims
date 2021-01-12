package api

import (
	network "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/bootstrap"
	ca "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/ca"
	transfer "github.com/MemeLabs/go-ppspp/pkg/apis/transfer/v1"
)

// PeerClient ...
type PeerClient interface {
	Bootstrap() *bootstrap.PeerServiceClient
	CA() *ca.CAPeerClient
	Transfer() *transfer.TransferPeerClient
	Network() *network.NetworkPeerClient
}
