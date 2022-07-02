// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package dao

import (
	"crypto/ed25519"
	"crypto/rand"
	"encoding/base64"
	"encoding/binary"
	"errors"
	"fmt"

	"github.com/MemeLabs/strims/pkg/apis/type/certificate"
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	videov1 "github.com/MemeLabs/strims/pkg/apis/video/v1"
	"github.com/MemeLabs/strims/pkg/kv"
	"google.golang.org/protobuf/proto"
)

var VideoChannels = NewTable(
	videoChannelNS,
	&TableOptions[videov1.VideoChannel, *videov1.VideoChannel]{
		ObserveChange: func(m, p *videov1.VideoChannel) proto.Message {
			return &videov1.VideoChannelChangeEvent{VideoChannel: m}
		},
		ObserveDelete: func(m *videov1.VideoChannel) proto.Message {
			return &videov1.VideoChannelDeleteEvent{VideoChannel: m}
		},
	},
)

const (
	_ byte = iota
	videoChannelLocalShare
	videoChannelRemoteShare
)

var videoChannelsByUniqueIndex = NewUniqueIndex(videoChannelKeyNS, VideoChannels, func(v *videov1.VideoChannel) []byte {
	var key []byte
	switch o := v.Owner.(type) {
	case *videov1.VideoChannel_LocalShare_:
		key = append(key, videoChannelLocalShare)
		key = append(key, CertificateRoot(o.LocalShare.Certificate).Key...)
		key = append(key, o.LocalShare.Certificate.Key...)
		return key
	case *videov1.VideoChannel_RemoteShare_:
		key = append(key, videoChannelRemoteShare)
		key = append(key, o.RemoteShare.NetworkKey...)
		key = append(key, o.RemoteShare.ServiceKey...)
		return key
	default:
		return nil
	}
}, nil)

// GetVideoChannelIDByOwnerCert ...
func GetVideoChannelIDByOwnerCert(s kv.Store, cert *certificate.Certificate) (uint64, error) {
	var key []byte
	key = append(key, videoChannelLocalShare)
	key = append(key, CertificateRoot(cert).Key...)
	key = append(key, cert.Key...)

	res, err := videoChannelsByUniqueIndex.Get(s, key)
	return res.GetId(), err
}

// GetVideoChannelByStreamKey ...
func GetVideoChannelByStreamKey(s kv.Store, key string) (*videov1.VideoChannel, error) {
	id, signature, err := ParseVideoChannelStreamKey(key)
	if err != nil {
		return nil, fmt.Errorf("parsing stream key: %w", err)
	}

	v, err := VideoChannels.Get(s, id)
	if err != nil {
		return nil, errors.New("channel not found")
	}

	var ownerKey []byte
	switch o := v.Owner.(type) {
	case *videov1.VideoChannel_Local_:
		ownerKey = o.Local.GetAuthKey()
	case *videov1.VideoChannel_LocalShare_:
		ownerKey = o.LocalShare.GetCertificate().GetKey()
	default:
		return nil, errors.New("channel not found")
	}
	if !ed25519.Verify(ownerKey, v.Token, signature) {
		return nil, errors.New("invalid stream token signature")
	}

	return v, nil
}

// NewVideoChannel ...
func NewVideoChannel(g IDGenerator) (*videov1.VideoChannel, error) {
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

	return &videov1.VideoChannel{
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
