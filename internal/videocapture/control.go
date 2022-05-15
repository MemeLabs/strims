// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package videocapture

import (
	"context"
	"errors"
	"sync"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/directory"
	"github.com/MemeLabs/strims/internal/network"
	"github.com/MemeLabs/strims/internal/transfer"
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"github.com/MemeLabs/strims/pkg/chunkstream"
	"github.com/MemeLabs/strims/pkg/ioutil"
	"github.com/MemeLabs/strims/pkg/logutil"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/ppspp/integrity"
	"github.com/MemeLabs/strims/pkg/timeutil"
	"go.uber.org/zap"
)

// errors ...
var (
	ErrIDNotFound = errors.New("id not found")
)

type Control interface {
	Open(mimeType string, directorySnippet *networkv1directory.ListingSnippet, networkKeys [][]byte) (transfer.ID, error)
	OpenWithSwarmWriterOptions(mimeType string, directorySnippet *networkv1directory.ListingSnippet, networkKeys [][]byte, options ppspp.WriterOptions) (transfer.ID, error)
	Update(id transfer.ID, directorySnippet *networkv1directory.ListingSnippet) error
	Append(id transfer.ID, b []byte, segmentEnd bool) error
	Close(id transfer.ID) error
}

// NewControl ...
func NewControl(
	ctx context.Context,
	logger *zap.Logger,
	transfer transfer.Control,
	directory directory.Control,
	network network.Control,
) Control {
	return &control{
		ctx:       ctx,
		logger:    logger,
		directory: directory,
		network:   network,
		transfer:  transfer,
		streams:   transferStreamMap{},
	}
}

// Control ...
type control struct {
	ctx       context.Context
	logger    *zap.Logger
	directory directory.Control
	network   network.Control
	transfer  transfer.Control

	lock    sync.Mutex
	streams transferStreamMap
}

// Open ...
func (c *control) Open(mimeType string, directorySnippet *networkv1directory.ListingSnippet, networkKeys [][]byte) (transfer.ID, error) {
	key, err := dao.GenerateKey()
	if err != nil {
		return transfer.ID{}, err
	}

	options := ppspp.WriterOptions{
		SwarmOptions: ppspp.SwarmOptions{
			ChunkSize:          1024,
			ChunksPerSignature: 32,
			LiveWindow:         32 * 1024,
			Integrity: integrity.VerifierOptions{
				ProtectionMethod:       integrity.ProtectionMethodMerkleTree,
				MerkleHashTreeFunction: integrity.MerkleHashTreeFunctionBLAKE2B256,
				LiveSignatureAlgorithm: integrity.LiveSignatureAlgorithmED25519,
			},
		},
		Key: key,
	}

	return c.OpenWithSwarmWriterOptions(mimeType, directorySnippet, networkKeys, options)
}

// OpenWithSwarmWriterOptions ...
func (c *control) OpenWithSwarmWriterOptions(mimeType string, directorySnippet *networkv1directory.ListingSnippet, networkKeys [][]byte, options ppspp.WriterOptions) (transfer.ID, error) {
	w, err := ppspp.NewWriter(options)
	if err != nil {
		return transfer.ID{}, err
	}

	cw, err := chunkstream.NewWriterSize(w, chunkstream.DefaultSize)
	if err != nil {
		return transfer.ID{}, err
	}

	transferID := c.transfer.Add(w.Swarm(), []byte{})
	for _, k := range networkKeys {
		c.transfer.Publish(transferID, k)
		c.logger.Debug(
			"publishing transfer",
			logutil.ByteHex("transfer", transferID[:]),
			logutil.ByteHex("network", k),
		)
	}

	s := &stream{
		transferID:  transferID,
		startTime:   timeutil.Now(),
		mimeType:    mimeType,
		networkKeys: networkKeys,
		key:         options.Key,
		swarm:       w.Swarm(),
		w:           cw,
	}

	dao.SignMessage(directorySnippet, s.key)
	c.directory.PushSnippet(s.swarm.ID(), directorySnippet)

	c.publishStream(s)

	c.lock.Lock()
	defer c.lock.Unlock()
	c.streams[transferID] = s

	return transferID, nil
}

// Update ...
func (c *control) Update(id transfer.ID, directorySnippet *networkv1directory.ListingSnippet) error {
	c.lock.Lock()
	defer c.lock.Unlock()

	s, ok := c.streams[id]
	if !ok {
		return ErrIDNotFound
	}

	dao.SignMessage(directorySnippet, s.key)
	c.directory.PushSnippet(s.swarm.ID(), directorySnippet)

	return nil
}

// Append ...
func (c *control) Append(id transfer.ID, b []byte, segmentEnd bool) error {
	c.lock.Lock()
	defer c.lock.Unlock()

	s, ok := c.streams[id]
	if !ok {
		return ErrIDNotFound
	}

	if _, err := s.w.Write(b); err != nil {
		return err
	}
	if segmentEnd {
		if err := s.w.Flush(); err != nil {
			return err
		}
	}
	return nil
}

// Close ...
func (c *control) Close(id transfer.ID) error {
	c.lock.Lock()
	defer c.lock.Unlock()

	s, ok := c.streams[id]
	if !ok {
		return ErrIDNotFound
	}

	delete(c.streams, id)
	c.transfer.Remove(s.transferID)

	c.directory.DeleteSnippet(s.swarm.ID())

	for _, k := range s.networkKeys {
		go c.unpublishDirectoryListing(k, s)
	}

	return nil
}

func (c *control) publishStream(s *stream) {
	for _, k := range s.networkKeys {
		go func(k []byte) {
			id, err := c.publishDirectoryListing(k, s)
			if err != nil {
				c.logger.Debug(
					"publishing video capture failed",
					logutil.ByteHex("network", k),
					zap.Error(err),
				)
			}

			s.directoryID = id
		}(k)
	}
}

func (c *control) publishDirectoryListing(networkKey []byte, s *stream) (uint64, error) {
	listing := &networkv1directory.Listing{
		Content: &networkv1directory.Listing_Media_{
			Media: &networkv1directory.Listing_Media{
				MimeType: s.mimeType,
				SwarmUri: s.swarm.URI().String(),
			},
		},
	}

	return c.directory.Publish(c.ctx, listing, networkKey)
}

func (c *control) unpublishDirectoryListing(networkKey []byte, s *stream) error {
	return c.directory.Unpublish(c.ctx, s.directoryID, networkKey)
}

type stream struct {
	transferID  transfer.ID
	startTime   timeutil.Time
	directoryID uint64
	mimeType    string
	networkKeys [][]byte
	key         *key.Key
	swarm       *ppspp.Swarm
	w           ioutil.WriteFlusher
}

type transferStreamMap map[transfer.ID]*stream
