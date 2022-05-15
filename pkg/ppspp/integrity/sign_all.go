// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package integrity

import (
	"errors"
	"io"
	"sync"

	swarmpb "github.com/MemeLabs/strims/pkg/apis/type/swarm"
	"github.com/MemeLabs/strims/pkg/binmap"
	"github.com/MemeLabs/strims/pkg/bufioutil"
	"github.com/MemeLabs/strims/pkg/ioutil"
	"github.com/MemeLabs/strims/pkg/ppspp/codec"
	"github.com/MemeLabs/strims/pkg/timeutil"
)

// errors ...
var (
	ErrMissingChunkSignature = errors.New("missing chunk signature")
	ErrInvalidChunkSignature = errors.New("invalid chunk signature")
)

// SignAllOptions ...
type SignAllOptions struct {
	LiveDiscardWindow int
	ChunkSize         int
	Verifier          SignatureVerifier
	Hash              HashFunc
}

// NewSignAllSwarmVerifier ...
func NewSignAllSwarmVerifier(o *SignAllOptions) *SignAllSwarmVerifier {
	return &SignAllSwarmVerifier{
		mask:              uint64(o.LiveDiscardWindow - 1),
		size:              binmap.Bin(o.LiveDiscardWindow * 2),
		timestamps:        make([]timeutil.Time, o.LiveDiscardWindow),
		signatures:        make([]byte, o.LiveDiscardWindow*o.Verifier.Size()),
		chunkSize:         o.ChunkSize,
		signatureVerifier: o.Verifier,
	}
}

// SignAllSwarmVerifier ...
type SignAllSwarmVerifier struct {
	lock              sync.Mutex
	mask              uint64
	size              binmap.Bin
	head, tail        binmap.Bin
	timestamps        []timeutil.Time
	signatures        []byte
	chunkSize         int
	signatureVerifier SignatureVerifier
}

// WriteIntegrity ...
func (v *SignAllSwarmVerifier) WriteIntegrity(b binmap.Bin, m *binmap.Map, w Writer) (int, error) {
	v.lock.Lock()
	defer v.lock.Unlock()

	var n int

	for l, r := b.BaseLeft(), b.BaseRight(); l <= r; l += 2 {
		if l < v.tail || l >= v.head {
			return n, ErrMissingChunkSignature
		}

		i := uint64(l>>1) & v.mask
		vs := uint64(v.signatureVerifier.Size())

		nn, err := w.WriteSignedIntegrity(codec.SignedIntegrity{
			Address:   codec.Address(l),
			Timestamp: codec.Timestamp{Time: v.timestamps[i]},
			Signature: v.signatures[i*vs : (i+1)*vs],
		})
		n += nn
		if err != nil {
			return n, err
		}
	}

	return n, nil
}

func (v *SignAllSwarmVerifier) storeSignature(b binmap.Bin, ts timeutil.Time, sig []byte) {
	v.lock.Lock()
	defer v.lock.Unlock()

	if b >= v.head {
		v.head = b + 2
		if v.head > v.size {
			v.tail = v.head - v.size
		}
	}

	i := uint64(b>>1) & v.mask
	vs := uint64(v.signatureVerifier.Size())

	v.timestamps[i] = ts
	copy(v.signatures[i*vs:(i+1)*vs], sig)
}

// ChannelVerifier ...
func (v *SignAllSwarmVerifier) ChannelVerifier() ChannelVerifier {
	return newSignAllChannelVerifier(v)
}

func (v *SignAllSwarmVerifier) ImportCache(c *swarmpb.Cache) error {
	v.lock.Lock()
	defer v.lock.Unlock()

	ic := c.Integrity.SignAllIntegrity
	if ic == nil {
		return errors.New("no supported integrity cache found")
	}

	for i, t := range ic.Timestamps {
		chunk := c.Data[i*v.chunkSize : (i+1)*v.chunkSize]
		sig := ic.Signatures[i*v.signatureVerifier.Size() : (i+1)*v.signatureVerifier.Size()]

		if !v.signatureVerifier.Verify(timeutil.Time(t), chunk, sig) {
			return ErrInvalidChunkSignature
		}
	}

	for i, t := range ic.Timestamps {
		v.timestamps[i] = timeutil.Time(t)
	}
	copy(v.signatures, ic.Signatures)

	return nil
}

func (v *SignAllSwarmVerifier) ExportCache() *swarmpb.Cache_Integrity {
	v.lock.Lock()
	defer v.lock.Unlock()

	c := &swarmpb.Cache_SignAllIntegrity{}

	for _, t := range v.timestamps {
		if t == 0 {
			break
		}
		c.Timestamps = append(c.Timestamps, int64(t))
	}
	c.Signatures = v.signatures[:len(c.Timestamps)*v.signatureVerifier.Size()]

	return &swarmpb.Cache_Integrity{SignAllIntegrity: c}
}

func newSignAllChannelVerifier(v *SignAllSwarmVerifier) *SignAllChannelVerifier {
	return &SignAllChannelVerifier{
		chunkVerifier: SignAllChunkVerifier{
			bin:           binmap.None,
			swarmVerifier: v,
		},
	}
}

// SignAllChannelVerifier ...
type SignAllChannelVerifier struct {
	chunkVerifier SignAllChunkVerifier
}

// ChunkVerifier ...
func (v *SignAllChannelVerifier) ChunkVerifier(b binmap.Bin) ChunkVerifier {
	return &v.chunkVerifier
}

// SignAllChunkVerifier ...
type SignAllChunkVerifier struct {
	swarmVerifier *SignAllSwarmVerifier
	bin           binmap.Bin
	timestamps    []timeutil.Time
	signatures    []byte
}

// SetSignedIntegrity ...
func (v *SignAllChunkVerifier) SetSignedIntegrity(b binmap.Bin, ts timeutil.Time, sig []byte) {
	if v.bin == binmap.None {
		v.bin = b
	}
	v.timestamps = append(v.timestamps, ts)
	v.signatures = append(v.signatures, sig...)
}

// SetIntegrity ...
func (v *SignAllChunkVerifier) SetIntegrity(b binmap.Bin, d []byte) {}

func (v *SignAllChunkVerifier) verify(b binmap.Bin, d []byte) (bool, error) {
	if b.BaseLeft() != v.bin {
		return false, ErrMissingChunkSignature
	}

	l := int(b.BaseLength())
	if l > len(v.timestamps) {
		return false, ErrMissingChunkSignature
	}
	for i := 0; i < l; i++ {
		ts := v.timestamps[i]
		chunk := d[i*v.swarmVerifier.chunkSize : (i+1)*v.swarmVerifier.chunkSize]
		sig := v.signatures[i*v.swarmVerifier.signatureVerifier.Size() : (i+1)*v.swarmVerifier.signatureVerifier.Size()]

		if !v.swarmVerifier.signatureVerifier.Verify(ts, chunk, sig) {
			return false, ErrInvalidChunkSignature
		}
		v.swarmVerifier.storeSignature(v.bin+binmap.Bin(i*2), ts, sig)
	}

	return true, nil
}

// Verify ...
func (v *SignAllChunkVerifier) Verify(b binmap.Bin, d []byte) (bool, error) {
	if verified, err := v.verify(b, d); !verified {
		return false, err
	}

	v.bin = binmap.None
	v.timestamps = v.timestamps[:0]
	v.signatures = v.signatures[:0]

	return true, nil
}

// SignAllWriterOptions ...
type SignAllWriterOptions struct {
	Verifier  *SignAllSwarmVerifier
	Writer    ioutil.WriteFlusher
	ChunkSize int
	Signer    SignatureSigner
}

// NewSignAllWriter ...
func NewSignAllWriter(o *SignAllWriterOptions) *SignAllWriter {
	sw := &signAllWriter{
		swarmVerifier:   o.Verifier,
		signatureSigner: o.Signer,
		w:               o.Writer,
	}
	return &SignAllWriter{
		bw: bufioutil.NewWriter(sw, o.ChunkSize),
	}
}

// SignAllWriter ...
type SignAllWriter struct {
	bw *bufioutil.Writer
}

// Write ...
func (w *SignAllWriter) Write(p []byte) (int, error) {
	return w.bw.Write(p)
}

// Flush ...
func (w *SignAllWriter) Flush() error {
	return w.bw.Flush()
}

type signAllWriter struct {
	b               binmap.Bin
	swarmVerifier   *SignAllSwarmVerifier
	signatureSigner SignatureSigner
	w               io.Writer
}

func (w *signAllWriter) Write(p []byte) (int, error) {
	ts := timeutil.Now()
	sig := w.signatureSigner.Sign(ts, p)
	w.swarmVerifier.storeSignature(w.b, ts, sig)

	w.b += 2

	return w.w.Write(p)
}
