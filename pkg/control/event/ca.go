package event

import "github.com/MemeLabs/go-ppspp/pkg/pb"

// CARenewNetworkCertError ...
type CARenewNetworkCertError struct {
	Network *pb.Network
	Error   error
}

// CARenewNetworkCert ...
type CARenewNetworkCert struct {
	Network     *pb.Network
	Certificate *pb.Certificate
}
