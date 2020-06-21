package driver

import "github.com/MemeLabs/go-ppspp/pkg/rpc"

type ClientOptions struct {
	VPNServerAddr string
}

// Driver ...
type Driver interface {
	Client(*ClientOptions) *rpc.Client
	Close()
}
