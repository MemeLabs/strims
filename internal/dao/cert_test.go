package dao

import (
	"crypto/ed25519"
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/stretchr/testify/assert"
)

func generateED25519Key(t *testing.T) *key.Key {
	t.Helper()
	pub, priv, err := ed25519.GenerateKey(nil)
	assert.Nil(t, err, "failed to generate ed25519 key")

	return &key.Key{
		Type:    key.KeyType_KEY_TYPE_ED25519,
		Public:  pub,
		Private: priv,
	}
}

type testcase struct {
	req  *certificate.CertificateRequest
	cert *certificate.Certificate
	key  *key.Key
	err  error
}

func buildTestCases(t *testing.T) (map[string]testcase, error) {
	t.Helper()
	k := generateED25519Key(t)

	successCsr, err := NewCertificateRequest(k, certificate.KeyUsage_KEY_USAGE_SIGN)
	assert.Nil(t, err, "failed to create success CSR")

	invalidLenCsr, err := NewCertificateRequest(k, certificate.KeyUsage_KEY_USAGE_SIGN)
	assert.Nil(t, err, "failed to create invalid CSR")

	invalidLenCsr.Key = k.Public[:len(k.Public)-5]

	invalidTypeCsr, err := NewCertificateRequest(k, certificate.KeyUsage_KEY_USAGE_SIGN)
	assert.Nil(t, err, "failed to create invalid key type CSR")

	invalidTypeCsr.KeyType = key.KeyType_KEY_TYPE_X25519

	successCert, err := SignCertificateRequest(successCsr, defaultCertTTL, k)
	assert.Nil(t, err, "failed to sign success CSR")

	invalidLenCert, err := SignCertificateRequest(successCsr, defaultCertTTL, k)
	assert.Nil(t, err, "failed to sign success CSR")
	invalidLenCert.Key = k.Private[:len(k.Private)-5]

	invalidTypeCert, err := SignCertificateRequest(successCsr, defaultCertTTL, k)
	assert.Nil(t, err, "failed to sign success CSR")
	invalidTypeCert.KeyType = key.KeyType_KEY_TYPE_X25519

	tcs := map[string]testcase{
		"success": {
			req:  successCsr,
			key:  k,
			cert: successCert,
			err:  nil,
		},
		"invalid key length": {
			req:  invalidLenCsr,
			key:  &key.Key{Type: k.Type, Private: k.Private[:len(k.Private)-5], Public: k.Public},
			cert: invalidLenCert,
			err:  ErrInvalidKeyLength,
		},
		"invalid key type (x25519)": {
			req:  invalidTypeCsr,
			key:  &key.Key{Type: key.KeyType_KEY_TYPE_X25519, Private: k.Private, Public: k.Public},
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
			err := VerifyCertificateRequest(tc.req, certificate.KeyUsage_KEY_USAGE_SIGN)
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
