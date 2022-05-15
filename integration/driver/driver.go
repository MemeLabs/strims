// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

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
