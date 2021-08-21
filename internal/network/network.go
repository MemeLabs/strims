package network

import networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"

type network struct {
	network   *networkv1.Network
	peerCount int
}
