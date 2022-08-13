// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package chat

import (
	"github.com/MemeLabs/strims/internal/dao"
	chatv1 "github.com/MemeLabs/strims/pkg/apis/chat/v1"
	"github.com/MemeLabs/strims/pkg/protoutil"
	"go.uber.org/zap"
)

func newAssetPublisher(logger *zap.Logger, ew *protoutil.ChunkStreamWriter) *assetPublisher {
	return &assetPublisher{
		logger:      logger,
		eventWriter: ew,
		checksums:   map[uint64]uint32{},
	}
}

type assetPublisher struct {
	logger      *zap.Logger
	eventWriter *protoutil.ChunkStreamWriter
	checksums   map[uint64]uint32
	size        int
}

func (s *assetPublisher) Sync(
	config *chatv1.Server,
	icon *chatv1.ServerIcon,
	emotes []*chatv1.Emote,
	modifiers []*chatv1.Modifier,
	tags []*chatv1.Tag,
) error {
	b := &chatv1.AssetBundle{
		IsDelta: len(s.checksums) != 0,
	}

	removed := map[uint64]struct{}{}
	for id := range s.checksums {
		removed[id] = struct{}{}
	}

	for _, e := range emotes {
		delete(removed, e.Id)
		c := dao.CRC32Message(e)
		if c != s.checksums[e.Id] {
			s.checksums[e.Id] = c
			b.Emotes = append(b.Emotes, e)
		}
	}

	for _, e := range modifiers {
		delete(removed, e.Id)
		c := dao.CRC32Message(e)
		if c != s.checksums[e.Id] {
			s.checksums[e.Id] = c
			b.Modifiers = append(b.Modifiers, e)
		}
	}

	for _, e := range tags {
		delete(removed, e.Id)
		c := dao.CRC32Message(e)
		if c != s.checksums[e.Id] {
			s.checksums[e.Id] = c
			b.Tags = append(b.Tags, e)
		}
	}

	delete(removed, config.Id)
	c := dao.CRC32Message(config)
	if c != s.checksums[config.Id] {
		s.checksums[config.Id] = c
		b.Room = config.Room
	}

	if icon != nil {
		delete(removed, icon.Id)
		c := dao.CRC32Message(icon)
		if c != s.checksums[icon.Id] {
			s.checksums[icon.Id] = c
			b.Icon = icon.Image
		}
	}

	for id := range removed {
		b.RemovedIds = append(b.RemovedIds, id)
	}

	// TODO
	// n := s.eventWriter.Size(b)
	// if s.size + n > buffer size {
	// 	reset writer (clear swarm buffer)
	// 	build unified bundle
	// }
	// n.size += n

	s.logger.Debug("writing asset bundle", zap.Int("size", s.eventWriter.Size(b)))

	return s.eventWriter.Write(b)
}
