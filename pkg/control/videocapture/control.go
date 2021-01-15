package videocapture

import (
	"bytes"
	"context"
	"errors"
	"sync"
	"time"

	networkv1 "github.com/MemeLabs/go-ppspp/pkg/apis/network/v1"
	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
	"github.com/MemeLabs/go-ppspp/pkg/chunkstream"
	"github.com/MemeLabs/go-ppspp/pkg/control/directory"
	"github.com/MemeLabs/go-ppspp/pkg/control/network"
	"github.com/MemeLabs/go-ppspp/pkg/control/transfer"
	"github.com/MemeLabs/go-ppspp/pkg/dao"
	"github.com/MemeLabs/go-ppspp/pkg/ioutil"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
	"github.com/petar/GoLLRB/llrb"
	"go.uber.org/zap"
)

// errors ...
var (
	ErrIDNotFound = errors.New("id not found")
)

// NewControl ...
func NewControl(logger *zap.Logger, transfer *transfer.Control, directory *directory.Control, network *network.Control) *Control {
	return &Control{
		logger:    logger,
		directory: directory,
		network:   network,
		transfer:  transfer,
	}
}

// Control ...
type Control struct {
	logger    *zap.Logger
	events    chan interface{}
	directory *directory.Control
	network   *network.Control
	transfer  *transfer.Control

	lock    sync.Mutex
	streams llrb.LLRB
}

// Open ...
func (c *Control) Open(mimeType string, directorySnippet *networkv1.DirectoryListingSnippet, networkKeys [][]byte) ([]byte, error) {
	key, err := dao.GenerateKey()
	if err != nil {
		return nil, err
	}

	w, err := ppspp.NewWriter(ppspp.WriterOptions{
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
	})
	if err != nil {
		return nil, err
	}

	cw, err := chunkstream.NewWriterSize(w, chunkstream.MaxSize)
	if err != nil {
		return nil, err
	}

	transferID := c.transfer.Add(w.Swarm(), []byte{})
	for _, k := range networkKeys {
		c.transfer.Publish(transferID, k)
	}

	c.lock.Lock()
	defer c.lock.Unlock()

	c.streams.InsertNoReplace(&stream{
		transferID:       transferID,
		startTime:        time.Now(),
		mimeType:         mimeType,
		directorySnippet: directorySnippet,
		networkKeys:      networkKeys,
		key:              key,
		swarm:            w.Swarm(),
		w:                cw,
	})

	return transferID, nil
}

// Update ...
func (c *Control) Update(id []byte, directorySnippet *networkv1.DirectoryListingSnippet) error {
	c.lock.Lock()
	defer c.lock.Unlock()

	s, ok := c.streams.Get(&stream{transferID: id}).(*stream)
	if !ok {
		return ErrIDNotFound
	}

	s.directorySnippet = directorySnippet

	return nil
}

// Append ...
func (c *Control) Append(id []byte, b []byte, segmentEnd bool) error {
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
func (c *Control) Close(id []byte) error {
	c.lock.Lock()
	defer c.lock.Unlock()

	s, ok := c.streams.Delete(&stream{transferID: id}).(*stream)
	if !ok {
		return ErrIDNotFound
	}

	c.transfer.Remove(s.transferID)

	return nil
}

func (c *Control) publishDirectoryListing(networkKey []byte, s *stream) error {
	creator, ok := c.network.Certificate(networkKey)
	if !ok {
		return errors.New("network certificate not found")
	}

	listing := &networkv1.DirectoryListing{
		Creator:   creator,
		Timestamp: time.Now().Unix(),
		Snippet:   s.directorySnippet,
		Content: &networkv1.DirectoryListing_Media{
			Media: &networkv1.DirectoryListingMedia{
				StartedAt: s.startTime.Unix(),
				MimeType:  s.mimeType,
				SwarmUri:  s.swarm.URI().String(),
			},
		},
	}
	if err := dao.SignMessage(listing, s.key); err != nil {
		return err
	}

	return c.directory.Publish(context.Background(), listing, networkKey)
}

func (c *Control) unpublishDirectoryListing(networkKey []byte, s *stream) error {
	return c.directory.Unpublish(context.Background(), s.key.Public, networkKey)
}

type stream struct {
	transferID       []byte
	startTime        time.Time
	directorySnippet *networkv1.DirectoryListingSnippet
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
