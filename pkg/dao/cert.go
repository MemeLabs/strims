package dao

import (
	"crypto/ed25519"
	"crypto/rand"
	"encoding/binary"
	"errors"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
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

	reqBytes := serializeCertificateRequest(csr)

	switch key.Type {
	case pb.KeyType_KEY_TYPE_ED25519:
		if len(key.Private) != ed25519.PrivateKeySize {
			return nil, ErrInvalidKeyLength
		}
		csr.Signature = ed25519.Sign(key.Private, reqBytes)
	default:
		return nil, ErrUnsupportedKeyType
	}
	return csr, nil
}

// VerifyCertificateRequest ...
func VerifyCertificateRequest(csr *pb.CertificateRequest, usage pb.KeyUsage) error {
	if csr.KeyUsage&^uint32(usage) != 0 {
		return ErrUnsupportedKeyUsage
	}

	reqBytes := serializeCertificateRequest(csr)

	var validSig bool
	switch csr.KeyType {
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

	certBytes := serializeCertificate(cert)

	switch key.Type {
	case pb.KeyType_KEY_TYPE_ED25519:
		if len(key.Private) != ed25519.PrivateKeySize {
			return nil, ErrInvalidKeyLength
		}
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

		certBytes := serializeCertificate(cert)

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

// serializeCertificate returns a stable byte representation of a certificate
func serializeCertificate(cert *pb.Certificate) []byte {
	b := make([]byte, 24+len(cert.Key)+len(cert.SerialNumber))

	n := copy(b, cert.Key)
	binary.BigEndian.PutUint32(b[n:], uint32(cert.KeyType))
	n += 4
	binary.BigEndian.PutUint32(b[n:], cert.KeyUsage)
	n += 4
	binary.BigEndian.PutUint64(b[n:], cert.NotBefore)
	n += 8
	binary.BigEndian.PutUint64(b[n:], cert.NotAfter)
	n += 8
	n += copy(b[n:], cert.SerialNumber)

	return b
}

// serializeCertificateRequest returns a stable byte representation of a certificate request
func serializeCertificateRequest(csr *pb.CertificateRequest) []byte {
	b := make([]byte, 8+len(csr.Key))

	n := copy(b, csr.Key)
	binary.BigEndian.PutUint32(b[n:], uint32(csr.KeyType))
	n += 4
	binary.BigEndian.PutUint32(b[n:], csr.KeyUsage)
	n += 4

	return b
}
