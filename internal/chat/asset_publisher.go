// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package chat

import (
	"errors"
	"sync"

	"github.com/MemeLabs/strims/internal/dao"
	chatv1 "github.com/MemeLabs/strims/pkg/apis/chat/v1"
	"github.com/MemeLabs/strims/pkg/protoutil"
	"go.uber.org/zap"
	"google.golang.org/protobuf/reflect/protoreflect"
)

func newAssetPublisher(logger *zap.Logger, ew *protoutil.ChunkStreamWriter) *assetPublisher {
	return &assetPublisher{
		logger:      logger,
		eventWriter: ew,
		checksums:   map[checksumKey]uint32{},
	}
}

type assetPublisher struct {
	logger      *zap.Logger
	eventWriter *protoutil.ChunkStreamWriter
	mu          sync.Mutex
	checksums   map[checksumKey]uint32
	size        int
}

func (s *assetPublisher) Sync(
	unifiedUpdate bool,
	config *chatv1.Server,
	icon *chatv1.ServerIcon,
	emotes []*chatv1.Emote,
	modifiers []*chatv1.Modifier,
	tags []*chatv1.Tag,
) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	if unifiedUpdate {
		s.size = 0
		s.checksums = map[checksumKey]uint32{}
		s.eventWriter.Reset()
	}

	b := &chatv1.AssetBundle{
		IsDelta: len(s.checksums) != 0,
	}

	removed := map[checksumKey]struct{}{}
	for k := range s.checksums {
		removed[k] = struct{}{}
	}

	for _, e := range emotes {
		k := newChecksumKey(e)
		if e.Enable {
			delete(removed, k)
			c := dao.CRC32Message(e)
			if c != s.checksums[k] {
				s.checksums[k] = c
				b.Emotes = append(b.Emotes, e)
			}
		}
	}

	for _, e := range modifiers {
		k := newChecksumKey(e)
		delete(removed, k)
		c := dao.CRC32Message(e)
		if c != s.checksums[k] {
			s.checksums[k] = c
			b.Modifiers = append(b.Modifiers, e)
		}
	}

	for _, e := range tags {
		k := newChecksumKey(e)
		delete(removed, k)
		c := dao.CRC32Message(e)
		if c != s.checksums[k] {
			s.checksums[k] = c
			b.Tags = append(b.Tags, e)
		}
	}

	k := newChecksumKey(config)
	delete(removed, k)
	c := dao.CRC32Message(config)
	if c != s.checksums[k] {
		s.checksums[k] = c
		b.Room = config.Room
	}

	if icon != nil {
		k := newChecksumKey(icon)
		delete(removed, k)
		c := dao.CRC32Message(icon)
		if c != s.checksums[k] {
			s.checksums[k] = c
			b.Icon = icon.Image
		}
	}

	for k := range removed {
		b.RemovedIds = append(b.RemovedIds, k.id)
		delete(s.checksums, k)
	}

	n := s.eventWriter.Size(b)
	if s.size+n > assetWindowSize {
		if unifiedUpdate {
			return errors.New("asset bundle size limit exceeded")
		}
		return s.Sync(true, config, icon, emotes, modifiers, tags)
	}
	s.size += n

	s.logger.Debug(
		"writing asset bundle",
		zap.Int("size", n),
		zap.Int("totalSize", s.size),
	)

	return s.eventWriter.Write(b)
}

func newChecksumKey[V any, T dao.TableRecord[V]](m T) checksumKey {
	return checksumKey{m.ProtoReflect().Descriptor(), m.GetId()}
}

type checksumKey struct {
	d  protoreflect.Descriptor
	id uint64
}
