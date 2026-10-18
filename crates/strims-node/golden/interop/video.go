// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//go:build golden

package main

// Live video swarms on the node, for the video interop tests. With
// -network-control (and -peer-rpc), the verbs write a swarm as
// internal/videoingress writes a channel's, and read one through
// internal/videoegress's control as the frontend's Egress.OpenStream reads
// it, both through the network control's transfer control.
//
// Verbs (byte strings in hex, <key> a network key):
//
//	video-write-open <key seed> <key>   a writer swarm under the seed's key with the ingress swarm options
//	                                    (videoingress openWriter), under a plain chunkstream writer; its
//	                                    transfer under the empty salt published on the network -> <swarm uri>
//	video-write <file>                  write each record of the file to the writer and flush it, as the
//	                                    ingress writes a segment -> <records> <bytes> (totals so far)
//	video-read <uri> <key> <n> <file>   open a videoegress stream of the swarm on the network, then read
//	                                    <n> whole records into the file in the background -> <transfer id>
//
// A record file holds records back to back, each a 4-byte big-endian
// length and its bytes. A read drops the record it is in at a stream
// reset or a buffer underrun, as the frontend does, and counts it.
//
// Events:
//
//	video-read-done <file> <discontinuities>
//	video-read-error <file> <message hex>

import (
	"context"
	"encoding/binary"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"os"
	"strconv"
	"sync"
	"time"

	"github.com/MemeLabs/strims/internal/videoegress"
	"github.com/MemeLabs/strims/pkg/chunkstream"
	"github.com/MemeLabs/strims/pkg/httputil"
	"github.com/MemeLabs/strims/pkg/ppspp"
	"github.com/MemeLabs/strims/pkg/ppspp/integrity"
	"github.com/MemeLabs/strims/pkg/ppspp/store"
)

// videoReadTimeout bounds a video-read: its reader stops after it.
const videoReadTimeout = 5 * time.Minute

// videoIngressOptions are the swarm options of internal/videoingress's
// openWriter.
var videoIngressOptions = ppspp.SwarmOptions{
	ChunkSize:          1024,
	ChunksPerSignature: 32,
	StreamCount:        16,
	LiveWindow:         16 * 1024,
	Integrity: integrity.VerifierOptions{
		ProtectionMethod:       integrity.ProtectionMethodMerkleTree,
		MerkleHashTreeFunction: integrity.MerkleHashTreeFunctionBLAKE2B256,
		LiveSignatureAlgorithm: integrity.LiveSignatureAlgorithmED25519,
	},
}

type videoState struct {
	egressOnce sync.Once
	egress     videoegress.Control

	mu sync.Mutex
	// The writer of the last video-write-open, and what it took.
	writer  *chunkstream.Writer
	records int
	bytes   int
}

// videoEgress is internal/videoegress's control on the network state's
// store and transfer control, as internal/app builds it, started on first
// use.
func (n *node) videoEgress() videoegress.Control {
	v := &n.video
	v.egressOnce.Do(func() {
		s := n.network
		v.egress = videoegress.NewControl(context.Background(), n.logger, s.store, s.observers, httputil.NewMapServeMux(), s.profile, s.transfer)
		go v.egress.Run()
	})
	return v.egress
}

func (n *node) videoCommand(verb string, args []string) (string, error) {
	s := n.network
	if s == nil {
		return "", fmt.Errorf("%s: network control is off", verb)
	}
	need := func(k int) error {
		if len(args) < k {
			return fmt.Errorf("%s: want %d args, got %d", verb, k, len(args))
		}
		return nil
	}
	v := &n.video

	switch verb {
	case "video-write-open":
		if err := need(2); err != nil {
			return "", err
		}
		seed, err := unhex(args[0])
		if err != nil || len(seed) != 32 {
			return "", errors.New("bad key seed")
		}
		networkKey, err := unhex(args[1])
		if err != nil {
			return "", err
		}
		w, err := ppspp.NewWriter(ppspp.WriterOptions{SwarmOptions: videoIngressOptions, Key: keyFromSeed(seed)})
		if err != nil {
			return "", err
		}
		cw, err := chunkstream.NewWriterSize(w, chunkstream.DefaultSize)
		if err != nil {
			return "", err
		}
		v.mu.Lock()
		v.writer, v.records, v.bytes = cw, 0, 0
		v.mu.Unlock()
		s.transfer.Publish(s.transfer.Add(w.Swarm(), []byte{}), networkKey)
		return w.Swarm().URI().String(), nil

	case "video-write":
		if err := need(1); err != nil {
			return "", err
		}
		records, err := readRecords(args[0])
		if err != nil {
			return "", err
		}
		v.mu.Lock()
		defer v.mu.Unlock()
		if v.writer == nil {
			return "", errors.New("no video writer")
		}
		for _, r := range records {
			if _, err := v.writer.Write(r); err != nil {
				return "", fmt.Errorf("record %d: %w", v.records, err)
			}
			if err := v.writer.Flush(); err != nil {
				return "", fmt.Errorf("record %d: %w", v.records, err)
			}
			v.records++
			v.bytes += len(r)
		}
		return fmt.Sprintf("%d %d", v.records, v.bytes), nil

	case "video-read":
		if err := need(4); err != nil {
			return "", err
		}
		networkKey, err := unhex(args[1])
		if err != nil {
			return "", err
		}
		count, err := strconv.Atoi(args[2])
		if err != nil {
			return "", err
		}
		ctx, cancel := context.WithTimeout(context.Background(), videoReadTimeout)
		id, r, err := n.videoEgress().OpenStream(ctx, args[0], [][]byte{networkKey})
		if err != nil {
			cancel()
			return "", err
		}
		go func() {
			defer cancel()
			defer r.Close()
			readVideo(r, count, args[3])
		}()
		return hex.EncodeToString(id[:]), nil
	}
	return "", fmt.Errorf("unknown command %q", verb)
}

// readVideo reads count whole records from r into the file at path, and
// reports how the read ended.
func readVideo(r io.Reader, count int, path string) {
	var records [][]byte
	var record []byte
	discontinuities := 0
	b := make([]byte, 32*1024)
	for len(records) < count {
		n, err := r.Read(b)
		record = append(record, b[:n]...)
		switch err {
		case nil:
		case io.EOF:
			records = append(records, record)
			record = nil
		case store.ErrStreamReset, store.ErrBufferUnderrun:
			discontinuities++
			record = nil
		default:
			emit("video-read-error %s %s", path, hexs(err.Error()))
			return
		}
	}
	if err := writeRecords(path, records); err != nil {
		emit("video-read-error %s %s", path, hexs(err.Error()))
		return
	}
	emit("video-read-done %s %d", path, discontinuities)
}

func readRecords(path string) ([][]byte, error) {
	b, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	var records [][]byte
	for len(b) > 0 {
		if len(b) < 4 {
			return nil, errors.New("truncated record length")
		}
		l := int(binary.BigEndian.Uint32(b))
		if len(b)-4 < l {
			return nil, errors.New("truncated record")
		}
		records = append(records, b[4:4+l])
		b = b[4+l:]
	}
	return records, nil
}

func writeRecords(path string, records [][]byte) error {
	var b []byte
	for _, r := range records {
		b = binary.BigEndian.AppendUint32(b, uint32(len(r)))
		b = append(b, r...)
	}
	return os.WriteFile(path, b, 0o644)
}
