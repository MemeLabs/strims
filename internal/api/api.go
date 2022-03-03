package api

import (
	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	networkv1bootstrap "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/bootstrap"
	networkv1ca "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/ca"
	transferv1 "github.com/MemeLabs/go-ppspp/pkg/apis/transfer/v1"
)

// PeerClient ...
type PeerClient interface {
	Bootstrap() *networkv1bootstrap.PeerServiceClient
	CA() *networkv1ca.CAPeerClient
	Transfer() *transferv1.TransferPeerClient
	Network() *networkv1.NetworkPeerClient
}
