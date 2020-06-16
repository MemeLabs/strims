package dao

import (
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/tj/assert"
)

func TestVerifyCertificateRequest(t *testing.T) {
	key := &pb.Key{
		Type: pb.KeyType_KEY_TYPE_ED25519,
		Private: []byte(`-----BEGIN OPENSSH PRIVATE KEY-----
    b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
    QyNTUxOQAAACCFaBtey/Mas1Is8WWjq3QE9Gdgu4HlPZRO/JfmLccS6wAAAJj7aSXh+2kl
    4QAAAAtzc2gtZWQyNTUxOQAAACCFaBtey/Mas1Is8WWjq3QE9Gdgu4HlPZRO/JfmLccS6w
    AAAEAQN9fdacoXGr8u6QWshmzGKOJ+VUepzhMEp/MdpkFWH4VoG17L8xqzUizxZaOrdAT0
    Z2C7geU9lE78l+YtxxLrAAAADmpicHJhdHRAYXV0dW1uAQIDBAUGBw==
    -----END OPENSSH PRIVATE KEY-----
    `),
		Public: []byte(`
ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIIVoG17L8xqzUizxZaOrdAT0Z2C7geU9lE78l+YtxxLr jbpratt@autumn
    `),
	}

	tcs := map[string]struct {
		req *pb.CertificateRequest
		err error
	}{
		"success": {
			req: &pb.CertificateRequest{
				Key:      key.Public,
				KeyType:  key.Type,
				KeyUsage: 0,
			},
			err: nil,
		},
		"invalid key length": {
			req: &pb.CertificateRequest{
				Key:      key.Public[:len(key.Public)-5],
				KeyType:  key.Type,
				KeyUsage: 0,
			},
			err: ErrInvalidKeyLength,
		},
		"invalid key type (x25519)": {
			req: &pb.CertificateRequest{
				Key:      key.Public,
				KeyType:  pb.KeyType_KEY_TYPE_X25519,
				KeyUsage: 0,
			},
			err: ErrUnsupportedKeyType,
		},
	}
	for scenario, tc := range tcs {
		t.Run(scenario, func(t *testing.T) {
			err := VerifyCertificateRequest(tc.req, 0)
			// if this test case should error, check
			if tc.err != nil {
				assert.EqualError(t, err, tc.err.Error())
			}
		})
	}
}

func TestSignCertificateRequest(t *testing.T) {
	key := &pb.Key{
		Type: pb.KeyType_KEY_TYPE_ED25519,
		Private: []byte(`-----BEGIN OPENSSH PRIVATE KEY-----
    b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
    QyNTUxOQAAACCFaBtey/Mas1Is8WWjq3QE9Gdgu4HlPZRO/JfmLccS6wAAAJj7aSXh+2kl
    4QAAAAtzc2gtZWQyNTUxOQAAACCFaBtey/Mas1Is8WWjq3QE9Gdgu4HlPZRO/JfmLccS6w
    AAAEAQN9fdacoXGr8u6QWshmzGKOJ+VUepzhMEp/MdpkFWH4VoG17L8xqzUizxZaOrdAT0
    Z2C7geU9lE78l+YtxxLrAAAADmpicHJhdHRAYXV0dW1uAQIDBAUGBw==
    -----END OPENSSH PRIVATE KEY-----
    `),
		Public: []byte(`
ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIIVoG17L8xqzUizxZaOrdAT0Z2C7geU9lE78l+YtxxLr jbpratt@autumn
    `),
	}

	tcs := map[string]struct {
		req *pb.CertificateRequest
		key *pb.Key
		err error
	}{
		"success": {
			key: key,
			req: &pb.CertificateRequest{
				Key:      key.Public,
				KeyType:  key.Type,
				KeyUsage: 0,
			},
			err: nil,
		},
		"invalid key length": {
			key: &pb.Key{Type: key.Type, Private: key.Private[:len(key.Private)-5], Public: key.Public},
			req: &pb.CertificateRequest{
				Key:      key.Public,
				KeyType:  key.Type,
				KeyUsage: 0,
			},
			err: ErrInvalidKeyLength,
		},
		"invalid key type (x25519)": {
			key: &pb.Key{Type: pb.KeyType_KEY_TYPE_X25519, Private: key.Private, Public: key.Public},
			req: &pb.CertificateRequest{
				Key:      key.Public,
				KeyType:  key.Type,
				KeyUsage: 0,
			},
			err: ErrUnsupportedKeyType,
		},
	}
	for scenario, tc := range tcs {
		t.Run(scenario, func(t *testing.T) {
			_, err := SignCertificateRequest(tc.req, defaultCertTTL, tc.key)
			if tc.err != nil {
				assert.EqualError(t, err, tc.err.Error())
			}
		})
	}
}

/*
func TestVerifyCertificate(t *testing.T) {
	key := &pb.Key{
		Type: pb.KeyType_KEY_TYPE_ED25519,
		Private: []byte(`-----BEGIN OPENSSH PRIVATE KEY-----
    b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
    QyNTUxOQAAACCFaBtey/Mas1Is8WWjq3QE9Gdgu4HlPZRO/JfmLccS6wAAAJj7aSXh+2kl
    4QAAAAtzc2gtZWQyNTUxOQAAACCFaBtey/Mas1Is8WWjq3QE9Gdgu4HlPZRO/JfmLccS6w
    AAAEAQN9fdacoXGr8u6QWshmzGKOJ+VUepzhMEp/MdpkFWH4VoG17L8xqzUizxZaOrdAT0
    Z2C7geU9lE78l+YtxxLrAAAADmpicHJhdHRAYXV0dW1uAQIDBAUGBw==
    -----END OPENSSH PRIVATE KEY-----
    `),
		Public: []byte(`
ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIIVoG17L8xqzUizxZaOrdAT0Z2C7geU9lE78l+YtxxLr jbpratt@autumn
    `),
	}

	tcs := map[string]struct {
		cert *pb.Certificate
		err  error
	}{
		"success": {
			err: nil,
		},
		"invalid key length": {
			err: ErrInvalidKeyLength,
		},
		"invalid key type (x25519)": {
			err: ErrUnsupportedKeyType,
		},
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
