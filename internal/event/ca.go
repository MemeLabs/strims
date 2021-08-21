package event

import (
	network "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
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
