// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package debug

import (
	"context"
	"time"

	"github.com/MemeLabs/strims/internal/dao"
	"github.com/MemeLabs/strims/internal/directory"
	"github.com/MemeLabs/strims/internal/transfer"
	debugv1 "github.com/MemeLabs/strims/pkg/apis/debug/v1"
	networkv1directory "github.com/MemeLabs/strims/pkg/apis/network/v1/directory"
	"github.com/MemeLabs/strims/pkg/chunkstream"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/ppspp/integrity"
	"google.golang.org/protobuf/proto"
)

const mockServiceType = "mock_stream"

type mockStreamRunner struct {
	ID              uint64
	BitrateBytes    int
	SegmentInterval time.Duration
	Timeout         time.Duration
	NetworkKey      []byte

	w          *chunkstream.Writer
	transferID transfer.ID
	listingID  uint64
}

func (r *mockStreamRunner) Init(
	ctx context.Context,
	transfer transfer.Control,
	directory directory.Control,
) error {
	key, err := dao.GenerateKey()
	if err != nil {
		return err
	}

	w, err := ppspp.NewWriter(ppspp.WriterOptions{
		SwarmOptions: ppspp.SwarmOptions{
			ChunkSize:          1024,
			ChunksPerSignature: 32,
			StreamCount:        16,
			LiveWindow:         16 * 1024,
			Integrity: integrity.VerifierOptions{
				ProtectionMethod:       integrity.ProtectionMethodMerkleTree,
				MerkleHashTreeFunction: integrity.MerkleHashTreeFunctionBLAKE2B256,
				LiveSignatureAlgorithm: integrity.LiveSignatureAlgorithmED25519,
			},
		},
		Key: key,
	})
	if err != nil {
		return err
	}

	r.w, err = chunkstream.NewWriter(w)
	if err != nil {
		return err
	}

	listing := &networkv1directory.Listing{
		Content: &networkv1directory.Listing_Service_{
			Service: &networkv1directory.Listing_Service{
				Type:     mockServiceType,
				SwarmUri: w.Swarm().URI().String(),
			},
		},
	}
	r.listingID, err = directory.Publish(ctx, listing, r.NetworkKey)
	if err != nil {
		return err
	}

	r.transferID = transfer.Add(w.Swarm(), nil)
	transfer.Publish(r.transferID, r.NetworkKey)

	return nil
}

func (r *mockStreamRunner) Run(
	ctx context.Context,
	transfer transfer.Control,
	directory directory.Control,
) error {
	defer func() {
		transfer.Remove(r.transferID)
		directory.Unpublish(context.Background(), r.listingID, r.NetworkKey)
	}()

	if r.Timeout != 0 {
		var cancel context.CancelFunc
		ctx, cancel = context.WithTimeout(ctx, r.Timeout)
		defer cancel()
	}

	var err error
	var b []byte
	segment := &debugv1.MockStreamSegment{
		Id:      r.ID,
		Padding: make([]byte, r.BitrateBytes),
	}

	t := time.NewTicker(r.SegmentInterval)
	for {
		select {
		case ts := <-t.C:
			segment.Timestamp = int64(ts.UnixMicro())
			b, err = proto.MarshalOptions{}.MarshalAppend(b[:0], segment)
			if err != nil {
				return err
			}
			if _, err = r.w.Write(b); err != nil {
				return err
			}
			if err = r.w.Flush(); err != nil {
				return err
			}
		case <-ctx.Done():
			return nil
		}
	}
}
