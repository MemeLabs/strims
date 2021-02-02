package driver

import "github.com/MemeLabs/protobuf/pkg/rpc"

type ClientOptions struct {
	VPNServerAddr string
}

// Driver ...
type Driver interface {
	Client(*ClientOptions) *rpc.Client
	Close()
}
