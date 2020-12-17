package dao

import (
	"crypto/ed25519"
	"crypto/rand"
	"encoding/base64"
	"encoding/binary"
	"errors"
	"fmt"
	"strconv"

	"github.com/MemeLabs/go-ppspp/pkg/kv"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

const videoIngressChannelPrefix = "videoIngressChannel:"
const videoIngressChannelLocalShareIndexPrefix = "videoIngressChannelLocalShare"
const videoIngressChannelRemoteShareIndexPrefix = "videoIngressChannelRemoteShare:"

func prefixVideoIngressChannelKey(id uint64) string {
	return videoIngressChannelPrefix + strconv.FormatUint(id, 10)
}

// UpsertVideoIngressChannel ...
func UpsertVideoIngressChannel(s kv.RWStore, v *pb.VideoIngressChannel) error {
	return s.Update(func(tx kv.RWTx) error {
		if prefix, key, ok := getVideoIngressChannelUniqueKey(v); ok {
			if err := SetUniqueSecondaryIndex(s, prefix, key, v.Id); err != nil {
				return err
			}
		}

		return tx.Put(prefixVideoIngressChannelKey(v.Id), v)
	})
}

// DeleteVideoIngressChannel ...
func DeleteVideoIngressChannel(s kv.RWStore, id uint64) error {
	return s.Update(func(tx kv.RWTx) error {
		v, err := GetVideoIngressChannel(tx, id)
		if err != nil {
			return err
		}

		if prefix, key, ok := getVideoIngressChannelUniqueKey(v); ok {
			if err := DeleteSecondaryIndex(s, prefix, key, id); err != nil {
				return err
			}
		}

		return tx.Delete(prefixVideoIngressChannelKey(id))
	})
}

func getVideoIngressChannelUniqueKey(v *pb.VideoIngressChannel) (string, []byte, bool) {
	var key []byte
	switch o := v.Owner.(type) {
	case *pb.VideoIngressChannel_LocalShare_:
		key = append(key, GetRootCert(o.LocalShare.Certificate).Key...)
		key = append(key, o.LocalShare.Certificate.Key...)
		return videoIngressChannelLocalShareIndexPrefix, key, true
	case *pb.VideoIngressChannel_RemoteShare_:
		key = append(key, o.RemoteShare.NetworkKey...)
		key = append(key, o.RemoteShare.ServiceKey...)
		return videoIngressChannelRemoteShareIndexPrefix, key, true
	default:
		return "", nil, false
	}
}

// GetVideoIngressChannel ...
func GetVideoIngressChannel(s kv.Store, id uint64) (v *pb.VideoIngressChannel, err error) {
	v = &pb.VideoIngressChannel{}
	err = s.View(func(tx kv.Tx) error {
		return tx.Get(prefixVideoIngressChannelKey(id), v)
	})
	return
}

// GetVideoIngressChannelByStreamKey ...
func GetVideoIngressChannelByStreamKey(s kv.Store, key string) (*pb.VideoIngressChannel, error) {
	id, signature, err := ParseVideoIngressChannelStreamKey(key)
	if err != nil {
		return nil, fmt.Errorf("parsing stream key: %w", err)
	}

	v, err := GetVideoIngressChannel(s, id)
	if err != nil {
		return nil, errors.New("channel not found")
	}

	var ownerKey []byte
	switch o := v.Owner.(type) {
	case *pb.VideoIngressChannel_Local_:
		ownerKey = o.Local.GetAuthKey()
	case *pb.VideoIngressChannel_LocalShare_:
		ownerKey = o.LocalShare.GetCertificate().GetKey()
	default:
		return nil, errors.New("channel not found")
	}
	if !ed25519.Verify(ownerKey, v.Token, signature) {
		return nil, errors.New("invalid stream token signature")
	}

	return v, nil
}

// GetVideoIngressChannels ...
func GetVideoIngressChannels(s kv.Store) (v []*pb.VideoIngressChannel, err error) {
	v = []*pb.VideoIngressChannel{}
	err = s.View(func(tx kv.Tx) error {
		return tx.ScanPrefix(videoIngressChannelPrefix, &v)
	})
	return
}

// NewVideoIngressChannel ...
func NewVideoIngressChannel(g IDGenerator) (*pb.VideoIngressChannel, error) {
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

	return &pb.VideoIngressChannel{
		Id:    id,
		Key:   key,
		Token: token,
	}, nil
}

// ParseVideoIngressChannelStreamKey ...
func ParseVideoIngressChannelStreamKey(key string) (uint64, []byte, error) {
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

// FormatVideoIngressChannelStreamKey ...
func FormatVideoIngressChannelStreamKey(id uint64, token []byte, key *pb.Key) string {
	b := make([]byte, binary.MaxVarintLen64+ed25519.SignatureSize)
	n := binary.PutUvarint(b, id)
	n += copy(b[n:], ed25519.Sign(key.Private, token))
	return base64.RawURLEncoding.EncodeToString(b[:n])
}
