package dao

import (
	"crypto/ed25519"
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/tj/assert"
)

func generateED25519Key(t *testing.T) *pb.Key {
	t.Helper()
	pub, priv, err := ed25519.GenerateKey(nil)
	if err != nil {
		t.Fatal(err)
	}

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

// i don't exactly like this after writing it, though i want to
// cover all of these areas without managing to repeat myself to much
func buildTestCases(t *testing.T) (map[string]testcase, error) {
	t.Helper()
	key := generateED25519Key(t)
	successCsr, err := NewCertificateRequest(key, 0)
	if err != nil {
		return nil, err
	}

	invalidLenCsr, err := NewCertificateRequest(key, 0)
	if err != nil {
		return nil, err
	}
	invalidLenCsr.Key = key.Public[:len(key.Public)-5]

	invalidTypeCsr, err := NewCertificateRequest(key, 0)
	if err != nil {
		return nil, err
	}
	invalidTypeCsr.KeyType = pb.KeyType_KEY_TYPE_X25519

	tcs := map[string]testcase{
		"success": {
			req: successCsr,
			key: key,
			err: nil,
		},
		"invalid key length": {
			req: invalidLenCsr,
			key: &pb.Key{Type: key.Type, Private: key.Private[:len(key.Private)-5], Public: key.Public},
			err: ErrInvalidKeyLength,
		},
		"invalid key type (x25519)": {
			req: invalidTypeCsr,
			key: &pb.Key{Type: pb.KeyType_KEY_TYPE_X25519, Private: key.Private, Public: key.Public},
			err: ErrUnsupportedKeyType,
		},
	}
	return tcs, nil
}

// Doesn't seem needed as the test cases require this but its here
func TestNewCertificateRequest(t *testing.T) {
	tcs, err := buildTestCases(t)
	if err != nil {
		t.Fatal(err)
	}

	for scenario, tc := range tcs {
		t.Run(scenario, func(t *testing.T) {
			_, err := NewCertificateRequest(tc.key, 0)
			if tc.err != nil {
				assert.EqualError(t, err, tc.err.Error())
			} else {
				assert.NoError(t, err)
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
			err := VerifyCertificateRequest(tc.req, 0)
			if tc.err != nil {
				assert.EqualError(t, err, tc.err.Error())
			} else {
				assert.NoError(t, err)
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
			_, err := SignCertificateRequest(tc.req, defaultCertTTL, tc.key)
			if tc.err != nil {
				assert.EqualError(t, err, tc.err.Error())
			} else {
				assert.NoError(t, err)
			}

			// TODO: inspect the cert
		})
	}
}

/*
func TestVerifyCertificate(t *testing.T) {
	tcs, err := buildTestCases(t)
	if err != nil {
		t.Fatal(err)
	}

	for scenario, tc := range tcs {
		t.Run(scenario, func(t *testing.T) {
			if err := VerifyCertificate(tc.cert); err != nil {
				assert.EqualError(t, err, tc.err.Error())
			}
		})
	}
}
*/
