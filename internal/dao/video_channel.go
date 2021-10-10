package dao

import (
	"crypto/ed25519"
	"crypto/rand"
	"encoding/base64"
	"encoding/binary"
	"errors"
	"fmt"
	"strconv"

	"github.com/MemeLabs/go-ppspp/pkg/apis/type/certificate"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	video "github.com/MemeLabs/go-ppspp/pkg/apis/video/v1"
	"github.com/MemeLabs/go-ppspp/pkg/kv"
)

const videoChannelPrefix = "videoChannel:"
const videoChannelLocalShareIndexPrefix = "videoChannelLocalShare"
const videoChannelRemoteShareIndexPrefix = "videoChannelRemoteShare:"

func prefixVideoChannelKey(id uint64) string {
	return videoChannelPrefix + strconv.FormatUint(id, 10)
}

// UpsertVideoChannel ...
func UpsertVideoChannel(s kv.RWStore, v *video.VideoChannel) error {
	return s.Update(func(tx kv.RWTx) error {
		if prefix, key, ok := getVideoChannelUniqueKey(v); ok {
			if err := SetUniqueSecondaryIndex(s, prefix, key, v.Id); err != nil {
				return err
			}
		}

		return tx.Put(prefixVideoChannelKey(v.Id), v)
	})
}

// DeleteVideoChannel ...
func DeleteVideoChannel(s kv.RWStore, id uint64) error {
	return s.Update(func(tx kv.RWTx) error {
		v, err := GetVideoChannel(tx, id)
		if err != nil {
			return err
		}

		if prefix, key, ok := getVideoChannelUniqueKey(v); ok {
			if err := DeleteSecondaryIndex(s, prefix, key, id); err != nil {
				return err
			}
		}

		return tx.Delete(prefixVideoChannelKey(id))
	})
}

func getVideoChannelUniqueKey(v *video.VideoChannel) (string, []byte, bool) {
	var key []byte
	switch o := v.Owner.(type) {
	case *video.VideoChannel_LocalShare_:
		key = append(key, CertificateRoot(o.LocalShare.Certificate).Key...)
		key = append(key, o.LocalShare.Certificate.Key...)
		return videoChannelLocalShareIndexPrefix, key, true
	case *video.VideoChannel_RemoteShare_:
		key = append(key, o.RemoteShare.NetworkKey...)
		key = append(key, o.RemoteShare.ServiceKey...)
		return videoChannelRemoteShareIndexPrefix, key, true
	default:
		return "", nil, false
	}
}

// GetVideoChannel ...
func GetVideoChannel(s kv.Store, id uint64) (v *video.VideoChannel, err error) {
	v = &video.VideoChannel{}
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(prefixVideoChannelKey(id), v)
	})
	return
}

// GetVideoChannelByStreamKey ...
func GetVideoChannelByStreamKey(s kv.Store, key string) (*video.VideoChannel, error) {
	id, signature, err := ParseVideoChannelStreamKey(key)
	if err != nil {
		return nil, fmt.Errorf("parsing stream key: %w", err)
	}

	v, err := GetVideoChannel(s, id)
	if err != nil {
		return nil, errors.New("channel not found")
	}

	var ownerKey []byte
	switch o := v.Owner.(type) {
	case *video.VideoChannel_Local_:
		ownerKey = o.Local.GetAuthKey()
	case *video.VideoChannel_LocalShare_:
		ownerKey = o.LocalShare.GetCertificate().GetKey()
	default:
		return nil, errors.New("channel not found")
	}
	if !ed25519.Verify(ownerKey, v.Token, signature) {
		return nil, errors.New("invalid stream token signature")
	}

	return v, nil
}

// GetVideoChannelIDByOwnerCert ...
func GetVideoChannelIDByOwnerCert(s kv.Store, cert *certificate.Certificate) (uint64, error) {
	var key []byte
	key = append(key, CertificateRoot(cert).Key...)
	key = append(key, cert.Key...)
	return GetUniqueSecondaryIndex(s, videoChannelLocalShareIndexPrefix, key)
}

// GetVideoChannels ...
func GetVideoChannels(s kv.Store) (v []*video.VideoChannel, err error) {
	v = []*video.VideoChannel{}
	err = s.View(func(tx kv.Tx) error {
		return tx.ScanPrefix(videoChannelPrefix, &v)
	})
	return
}

// NewVideoChannel ...
func NewVideoChannel(g IDGenerator) (*video.VideoChannel, error) {
	id, err := g.GenerateID()
	if err != nil {
		return nil, err
	}

	key, err := GenerateKey()
	if err != nil {
		return nil, err
	}

	token := make([]byte, 32)
	if _, err := rand.Read(token); err != nil {
		return nil, err
	}

	return &video.VideoChannel{
		Id:    id,
		Key:   key,
		Token: token,
	}, nil
}

// ParseVideoChannelStreamKey ...
func ParseVideoChannelStreamKey(key string) (uint64, []byte, error) {
	b, err := base64.RawURLEncoding.DecodeString(key)
	if err != nil {
		return 0, nil, err
	}

	id, n := binary.Uvarint(b)

	sig := b[n:]
	if len(sig) != ed25519.SignatureSize {
		return 0, nil, errors.New("invalid key length")
	}

	return id, sig, nil
}

// FormatVideoChannelStreamKey ...
func FormatVideoChannelStreamKey(id uint64, token []byte, key *key.Key) string {
	b := make([]byte, binary.MaxVarintLen64+ed25519.SignatureSize)
	n := binary.PutUvarint(b, id)
	n += copy(b[n:], ed25519.Sign(key.Private, token))
	return base64.RawURLEncoding.EncodeToString(b[:n])
}
