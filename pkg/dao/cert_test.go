package dao

import (
	"crypto/ed25519"
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/stretchr/testify/assert"
)

func generateED25519Key(t *testing.T) *pb.Key {
	t.Helper()
	pub, priv, err := ed25519.GenerateKey(nil)
	assert.Nil(t, err, "failed to generate ed25519 key")

	return &pb.Key{
		Type:    pb.KeyType_KEY_TYPE_ED25519,
		Public:  pub,
		Private: priv,
	}
}

type testcase struct {
	req  *pb.CertificateRequest
	cert *pb.Certificate
	key  *pb.Key
	err  error
}

func buildTestCases(t *testing.T) (map[string]testcase, error) {
	t.Helper()
	key := generateED25519Key(t)

	successCsr, err := NewCertificateRequest(key, pb.KeyUsage_KEY_USAGE_SIGN)
	assert.Nil(t, err, "failed to create success CSR")

	invalidLenCsr, err := NewCertificateRequest(key, pb.KeyUsage_KEY_USAGE_SIGN)
	assert.Nil(t, err, "failed to create invalid CSR")

	invalidLenCsr.Key = key.Public[:len(key.Public)-5]

	invalidTypeCsr, err := NewCertificateRequest(key, pb.KeyUsage_KEY_USAGE_SIGN)
	assert.Nil(t, err, "failed to create invalid key type CSR")

	invalidTypeCsr.KeyType = pb.KeyType_KEY_TYPE_X25519

	successCert, err := SignCertificateRequest(successCsr, defaultCertTTL, key)
	assert.Nil(t, err, "failed to sign success CSR")

	invalidLenCert, err := SignCertificateRequest(successCsr, defaultCertTTL, key)
	assert.Nil(t, err, "failed to sign success CSR")
	invalidLenCert.Key = key.Private[:len(key.Private)-5]

	invalidTypeCert, err := SignCertificateRequest(successCsr, defaultCertTTL, key)
	assert.Nil(t, err, "failed to sign success CSR")
	invalidTypeCert.KeyType = pb.KeyType_KEY_TYPE_X25519

	tcs := map[string]testcase{
		"success": {
			req:  successCsr,
			key:  key,
			cert: successCert,
			err:  nil,
		},
		"invalid key length": {
			req:  invalidLenCsr,
			key:  &pb.Key{Type: key.Type, Private: key.Private[:len(key.Private)-5], Public: key.Public},
			cert: invalidLenCert,
			err:  ErrInvalidKeyLength,
		},
		"invalid key type (x25519)": {
			req:  invalidTypeCsr,
			key:  &pb.Key{Type: pb.KeyType_KEY_TYPE_X25519, Private: key.Private, Public: key.Public},
			cert: invalidTypeCert,
			err:  ErrUnsupportedKeyType,
		},
	}
	return tcs, nil
}

func TestNewCertificateRequest(t *testing.T) {
	tcs, err := buildTestCases(t)
	if err != nil {
		t.Fatal(err)
	}

	for scenario, tc := range tcs {
		t.Run(scenario, func(t *testing.T) {
			_, err := NewCertificateRequest(tc.key, 0)
			if tc.err != nil {
				if errs, ok := err.(Errors); ok {
					assert.True(t, errs.Includes(tc.err), "received errors: %s, expected: %s", errs, tc.err)
				} else {
					assert.EqualError(t, err, tc.err.Error())
				}
			} else {
				assert.Nil(t, err)
			}
		})
	}
}

func TestVerifyCertificateRequest(t *testing.T) {
	tcs, err := buildTestCases(t)
	if err != nil {
		t.Fatal(err)
	}

	for scenario, tc := range tcs {
		t.Run(scenario, func(t *testing.T) {
			err := VerifyCertificateRequest(tc.req, pb.KeyUsage_KEY_USAGE_SIGN)
			if tc.err != nil {
				assert.EqualError(t, err, tc.err.Error())
			} else {
				assert.Nil(t, err)
			}
		})
	}
}

func TestSignCertificateRequest(t *testing.T) {
	tcs, err := buildTestCases(t)
	if err != nil {
		t.Fatal(err)
	}

	for scenario, tc := range tcs {
		t.Run(scenario, func(t *testing.T) {
			cert, err := SignCertificateRequest(tc.req, defaultCertTTL, tc.key)
			if tc.err != nil {
				assert.EqualError(t, err, tc.err.Error())
			} else {
				assert.Nil(t, err)
				assert.NotNil(t, cert.GetKey())
			}
		})
	}
}

func TestVerifyCertificate(t *testing.T) {
	tcs, err := buildTestCases(t)
	if err != nil {
		t.Fatal(err)
	}

	for scenario, tc := range tcs {
		t.Run(scenario, func(t *testing.T) {
			err := VerifyCertificate(tc.cert)
			if tc.err != nil {
				if errs, ok := err.(Errors); ok {
					assert.True(t, errs.Includes(tc.err), "received errors: %s, expected: %s", errs, tc.err)
				} else {
					assert.EqualError(t, err, tc.err.Error())
				}
			} else {
				assert.Nil(t, err)
			}
		})
	}
}
