package driver

import "github.com/MemeLabs/go-ppspp/pkg/rpc"

// Driver ...
type Driver interface {
	Client() *rpc.Client
	Close()
}
