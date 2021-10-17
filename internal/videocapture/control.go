package videocapture

import (
	"bytes"
	"context"
	"errors"
	"sync"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	"github.com/MemeLabs/go-ppspp/internal/directory"
	"github.com/MemeLabs/go-ppspp/internal/network"
	"github.com/MemeLabs/go-ppspp/internal/transfer"
	networkv1directory "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/MemeLabs/go-ppspp/pkg/ioutil"
	"github.com/MemeLabs/go-ppspp/pkg/logutil"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/MemeLabs/go-ppspp/pkg/timeutil"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

// errors ...
var (
	ErrIDNotFound = errors.New("id not found")
)

type Control interface {
	Open(mimeType string, directorySnippet *networkv1directory.ListingSnippet, networkKeys [][]byte) ([]byte, error)
	OpenWithSwarmWriterOptions(mimeType string, directorySnippet *networkv1directory.ListingSnippet, networkKeys [][]byte, options ppspp.WriterOptions) ([]byte, error)
	Update(id []byte, directorySnippet *networkv1directory.ListingSnippet) error
	Append(id []byte, b []byte, segmentEnd bool) error
	Close(id []byte) error
}

// NewControl ...
func NewControl(
	logger *zap.Logger,
	transfer transfer.Control,
	directory directory.Control,
	network network.Control,
) Control {
	return &control{
		logger:    logger,
		directory: directory,
		network:   network,
		transfer:  transfer,
	}
}

// Control ...
type control struct {
	logger    *zap.Logger
	directory directory.Control
	network   network.Control
	transfer  transfer.Control

	lock    sync.Mutex
	streams llrb.LLRB
}

// Open ...
func (c *control) Open(mimeType string, directorySnippet *networkv1directory.ListingSnippet, networkKeys [][]byte) ([]byte, error) {
	key, err := dao.GenerateKey()
	if err != nil {
		return nil, err
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
func (c *control) OpenWithSwarmWriterOptions(mimeType string, directorySnippet *networkv1directory.ListingSnippet, networkKeys [][]byte, options ppspp.WriterOptions) ([]byte, error) {
	w, err := ppspp.NewWriter(options)
	if err != nil {
		return nil, err
	}

	cw, err := chunkstream.NewWriterSize(w, chunkstream.DefaultSize)
	if err != nil {
		return nil, err
	}

	transferID := c.transfer.Add(w.Swarm(), []byte{})
	for _, k := range networkKeys {
		c.transfer.Publish(transferID, k)
		c.logger.Debug(
			"publishing transfer",
			logutil.ByteHex("transfer", transferID),
			logutil.ByteHex("network", k),
		)
	}

	s := &stream{
		transferID:       transferID,
		startTime:        timeutil.Now(),
		mimeType:         mimeType,
		directorySnippet: directorySnippet,
		networkKeys:      networkKeys,
		key:              options.Key,
		swarm:            w.Swarm(),
		w:                cw,
	}

	c.publishStream(s)

	c.lock.Lock()
	defer c.lock.Unlock()
	c.streams.ReplaceOrInsert(s)

	return transferID, nil
}

// Update ...
func (c *control) Update(id []byte, directorySnippet *networkv1directory.ListingSnippet) error {
	c.lock.Lock()
	defer c.lock.Unlock()

	s, ok := c.streams.Get(&stream{transferID: id}).(*stream)
	if !ok {
		return ErrIDNotFound
	}

	s.directorySnippet = directorySnippet
	c.publishStream(s)
	// TODO: publish snippet to directory... snippet stream... need api

	return nil
}

// Append ...
func (c *control) Append(id []byte, b []byte, segmentEnd bool) error {
	c.lock.Lock()
	defer c.lock.Unlock()

	s, ok := c.streams.Get(&stream{transferID: id}).(*stream)
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
func (c *control) Close(id []byte) error {
	c.lock.Lock()
	defer c.lock.Unlock()

	s, ok := c.streams.Delete(&stream{transferID: id}).(*stream)
	if !ok {
		return ErrIDNotFound
	}

	c.transfer.Remove(s.transferID)

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

			// TODO: store id in stream directoryID
			_ = id
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

	return c.directory.Publish(context.Background(), listing, networkKey)
}

func (c *control) unpublishDirectoryListing(networkKey []byte, s *stream) error {
	return c.directory.Unpublish(context.Background(), s.directoryID, networkKey)
}

type stream struct {
	transferID       []byte
	startTime        timeutil.Time
	directoryID      uint64
	directorySnippet *networkv1directory.ListingSnippet
	mimeType         string
	networkKeys      [][]byte
	key              *key.Key
	swarm            *ppspp.Swarm
	w                ioutil.WriteFlusher
}

func (s *stream) Less(o llrb.Item) bool {
	if o, ok := o.(*stream); ok {
		return bytes.Compare(s.transferID, o.transferID) == -1
	}
	return !o.Less(s)
}
