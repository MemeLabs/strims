package dao

import (
	"crypto/ed25519"
	"crypto/rand"
	"errors"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"google.golang.org/protobuf/proto"
)

const defaultCertTTL = time.Hour * 24 * 365 * 2 // ~two years

// validation errors
var (
	ErrUnsupportedKeyType                 = errors.New("unsupported key type")
	ErrInvalidKeyLength                   = errors.New("invalid key length")
	ErrUnsupportedKeyUsage                = errors.New("unsupported key usage")
	ErrInvalidCertificateRequestSignature = errors.New("invalid certificate request signature")
	ErrInvalidSignature                   = errors.New("invalid certificate signature")
)

// NewCertificateRequest ...
func NewCertificateRequest(key *pb.Key, keyUsage pb.KeyUsage) (*pb.CertificateRequest, error) {
	csr := &pb.CertificateRequest{
		Key:      key.Public,
		KeyType:  key.Type,
		KeyUsage: uint32(keyUsage),
	}

	b, err := proto.Marshal(csr)
	if err != nil {
		return nil, err
	}
	switch key.Type {
	case pb.KeyType_KEY_TYPE_ED25519:
		csr.Signature = ed25519.Sign(key.Private, b)
	default:
		return nil, ErrUnsupportedKeyType
	}
	return csr, nil
}

// VerifyCertificateRequest ...
func VerifyCertificateRequest(csr *pb.CertificateRequest, usage pb.KeyUsage) error {
	temp := proto.Clone(csr).(*pb.CertificateRequest)
	temp.Signature = nil
	reqBytes, err := proto.Marshal(temp)
	if err != nil {
		return err
	}

	if csr.KeyUsage&^uint32(usage) != 0 {
		return ErrUnsupportedKeyUsage
	}

	var validSig bool
	switch temp.KeyType {
	case pb.KeyType_KEY_TYPE_ED25519:
		if len(csr.Key) != ed25519.PublicKeySize {
			return ErrInvalidKeyLength
		}
		validSig = ed25519.Verify(csr.Key, reqBytes, csr.Signature)
	default:
		return ErrUnsupportedKeyType
	}
	if !validSig {
		return ErrInvalidCertificateRequestSignature
	}

	return nil
}

// SignCertificateRequest ...
func SignCertificateRequest(
	csr *pb.CertificateRequest,
	validDuration time.Duration,
	key *pb.Key,
) (*pb.Certificate, error) {
	now := time.Now().UTC()
	cert := &pb.Certificate{
		Key:          csr.Key,
		KeyType:      csr.KeyType,
		KeyUsage:     csr.KeyUsage,
		NotBefore:    uint64(now.Unix()),
		NotAfter:     uint64(now.Add(validDuration).Unix()),
		SerialNumber: make([]byte, 16),
	}

	if _, err := rand.Read(cert.SerialNumber); err != nil {
		return nil, err
	}

	certBytes, err := proto.Marshal(cert)
	if err != nil {
		return nil, err
	}
	switch key.Type {
	case pb.KeyType_KEY_TYPE_ED25519:
		cert.Signature = ed25519.Sign(key.Private, certBytes)
	default:
		return nil, ErrUnsupportedKeyType
	}
	return cert, nil
}

// VerifyCertificate ...
func VerifyCertificate(cert *pb.Certificate) error {
	for parent := cert.GetParent(); cert != nil; cert, parent = parent, parent.GetParent() {
		// check that either the certificare is a self-signed root or has a valid
		// signing certificate as its parent.
		signingCert := cert
		if parent != nil {
			signingCert = parent
		}
		if signingCert.KeyUsage&uint32(pb.KeyUsage_KEY_USAGE_SIGN) == 0 {
			return ErrUnsupportedKeyUsage
		}

		temp := proto.Clone(cert).(*pb.Certificate)
		temp.Signature = nil
		temp.ParentOneof = nil
		certBytes, err := proto.Marshal(temp)
		if err != nil {
			return err
		}

		var validSig bool
		switch signingCert.KeyType {
		case pb.KeyType_KEY_TYPE_ED25519:
			if len(signingCert.Key) != ed25519.PublicKeySize {
				return ErrInvalidKeyLength
			}
			validSig = ed25519.Verify(signingCert.Key, certBytes, cert.Signature)
		default:
			return ErrUnsupportedKeyType
		}
		if !validSig {
			return ErrInvalidSignature
		}
	}
	return nil
}

// NewSelfSignedCertificate ...
func NewSelfSignedCertificate(
	key *pb.Key,
	usage pb.KeyUsage,
	validDuration time.Duration,
) (*pb.Certificate, error) {
	csr, err := NewCertificateRequest(key, usage)
	if err != nil {
		return nil, err
	}
	return SignCertificateRequest(csr, validDuration, key)
}

// GetRootCert returns the root certificate for a given certificate
func GetRootCert(cert *pb.Certificate) *pb.Certificate {
	for cert.GetParent() != nil {
		cert = cert.GetParent()
	}
	return cert
}

// CertIsExpired returns true if the cert NotBefore or NotAfter dates are violated
func CertIsExpired(cert *pb.Certificate) bool {
	now := uint64(time.Now().UTC().Unix())
	return cert.NotAfter <= now && cert.NotBefore >= now
}
