package dao

import (
	"crypto/ed25519"
	"crypto/rand"
	"encoding/binary"
	"errors"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	keyapi "github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
)

const certPredateDuration = time.Hour
const defaultCertTTL = time.Hour * 24 * 365 * 2 // ~two years

// validation errors
var (
	ErrUnsupportedKeyType                 = errors.New("unsupported key type")
	ErrInvalidKeyLength                   = errors.New("invalid key length")
	ErrUnsupportedKeyUsage                = errors.New("unsupported key usage")
	ErrInvalidCertificateRequestSignature = errors.New("invalid certificate request signature")
	ErrInvalidSignature                   = errors.New("invalid certificate signature")
	ErrNotBeforeRange                     = errors.New("not before limit exceeded")
	ErrNotAfterRange                      = errors.New("not after limit exceeded")
)

// CertificateRequestOption ...
type CertificateRequestOption func(*certificate.CertificateRequest)

// WithSubject ...
func WithSubject(subject string) CertificateRequestOption {
	return func(csr *certificate.CertificateRequest) {
		csr.Subject = subject
	}
}

// NewCertificateRequest ...
func NewCertificateRequest(key *keyapi.Key, usage certificate.KeyUsage, opts ...CertificateRequestOption) (*certificate.CertificateRequest, error) {
	csr := &certificate.CertificateRequest{
		Key:      key.Public,
		KeyType:  key.Type,
		KeyUsage: uint32(usage),
	}

	for _, opt := range opts {
		opt(csr)
	}

	reqBytes, _ := serializeCertificateRequest(csr)

	switch key.Type {
	case keyapi.KeyType_KEY_TYPE_ED25519:
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
func VerifyCertificateRequest(csr *certificate.CertificateRequest, usage certificate.KeyUsage) error {
	if csr.KeyUsage&^uint32(usage) != 0 {
		return ErrUnsupportedKeyUsage
	}

	reqBytes, _ := serializeCertificateRequest(csr)

	var validSig bool
	switch csr.KeyType {
	case keyapi.KeyType_KEY_TYPE_ED25519:
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
	csr *certificate.CertificateRequest,
	validDuration time.Duration,
	key *keyapi.Key,
) (*certificate.Certificate, error) {
	now := time.Now().UTC()
	cert := &certificate.Certificate{
		Key:          csr.Key,
		KeyType:      csr.KeyType,
		KeyUsage:     csr.KeyUsage,
		Subject:      csr.Subject,
		NotBefore:    uint64(now.Add(-certPredateDuration).Unix()),
		NotAfter:     uint64(now.Add(validDuration).Unix()),
		SerialNumber: make([]byte, 16),
	}

	if _, err := rand.Read(cert.SerialNumber); err != nil {
		return nil, err
	}

	certBytes, _ := serializeCertificate(cert)

	switch key.Type {
	case keyapi.KeyType_KEY_TYPE_ED25519:
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
func VerifyCertificate(cert *certificate.Certificate) error {
	var errs Errors
	now := time.Now()

	for parent := cert.GetParent(); cert != nil; cert, parent = parent, parent.GetParent() {
		// check that either the certificare is a self-signed root or has a valid
		// signing certificate as its parent.
		signingCert := cert
		if parent != nil {
			signingCert = parent
		}
		if signingCert.KeyUsage&uint32(certificate.KeyUsage_KEY_USAGE_SIGN) == 0 {
			errs = append(errs, ErrUnsupportedKeyUsage)
		}

		certBytes, _ := serializeCertificate(cert)

		var validSig bool
		switch signingCert.KeyType {
		case keyapi.KeyType_KEY_TYPE_ED25519:
			if len(signingCert.Key) != ed25519.PublicKeySize {
				errs = append(errs, ErrInvalidKeyLength)
				break
			}
			validSig = ed25519.Verify(signingCert.Key, certBytes, cert.Signature)
		default:
			errs = append(errs, ErrUnsupportedKeyType)
		}
		if !validSig {
			errs = append(errs, ErrInvalidSignature)
		}

		if now.Before(time.Unix(int64(cert.NotBefore), 0)) {
			errs = append(errs, ErrNotBeforeRange)
		}
		if now.After(time.Unix(int64(cert.NotAfter), 0)) {
			errs = append(errs, ErrNotAfterRange)
		}
	}

	if len(errs) != 0 {
		return errs
	}
	return nil
}

// NewSelfSignedCertificate ...
func NewSelfSignedCertificate(
	key *keyapi.Key,
	usage certificate.KeyUsage,
	validDuration time.Duration,
	opts ...CertificateRequestOption,
) (*certificate.Certificate, error) {
	csr, err := NewCertificateRequest(key, usage, opts...)
	if err != nil {
		return nil, err
	}
	return SignCertificateRequest(csr, validDuration, key)
}

// GetRootCert returns the root certificate for a given certificate
func GetRootCert(cert *certificate.Certificate) *certificate.Certificate {
	for cert.GetParent() != nil {
		cert = cert.GetParent()
	}
	return cert
}

// CertIsExpired returns true if the cert NotBefore or NotAfter dates are violated
func CertIsExpired(cert *certificate.Certificate) bool {
	now := uint64(time.Now().UTC().Unix())
	return cert.NotAfter <= now && cert.NotBefore >= now
}

// serializeCertificate returns a stable byte representation of a certificate
func serializeCertificate(cert *certificate.Certificate) ([]byte, int) {
	b := make([]byte, 24+len(cert.Key)+len(cert.Subject)+len(cert.SerialNumber))

	n := copy(b, cert.Key)
	binary.BigEndian.PutUint32(b[n:], uint32(cert.KeyType))
	n += 4
	binary.BigEndian.PutUint32(b[n:], cert.KeyUsage)
	n += 4
	n += copy(b[n:], []byte(cert.Subject))
	binary.BigEndian.PutUint64(b[n:], cert.NotBefore)
	n += 8
	binary.BigEndian.PutUint64(b[n:], cert.NotAfter)
	n += 8
	n += copy(b[n:], cert.SerialNumber)

	return b, n
}

// serializeCertificateRequest returns a stable byte representation of a certificate request
func serializeCertificateRequest(csr *certificate.CertificateRequest) ([]byte, int) {
	b := make([]byte, 8+len(csr.Key)+len([]byte(csr.Subject)))

	n := copy(b, csr.Key)
	binary.BigEndian.PutUint32(b[n:], uint32(csr.KeyType))
	n += 4
	binary.BigEndian.PutUint32(b[n:], csr.KeyUsage)
	n += 4
	n += copy(b[n:], []byte(csr.Subject))

	return b, n
}
