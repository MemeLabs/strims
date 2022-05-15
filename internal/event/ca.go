// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package event

import (
	network "github.com/MemeLabs/strims/pkg/apis/network/v1"
	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
)

// CARenewNetworkCertError ...
type CARenewNetworkCertError struct {
	Network *network.Network
	Error   error
}

// CARenewNetworkCert ...
type CARenewNetworkCert struct {
	Network     *network.Network
	Certificate *certificate.Certificate
}
