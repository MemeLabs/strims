// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package integrity

import (
	"crypto/ed25519"
	"crypto/sha1"
	"crypto/sha256"
	"crypto/sha512"
	"errors"
	"hash"
	"math/bits"

	"github.com/zeebo/blake3"
	"golang.org/x/crypto/blake2b"
	"golang.org/x/crypto/sha3"

	swarmpb "github.com/MemeLabs/strims/pkg/apis/type/swarm"
	"github.com/MemeLabs/strims/pkg/binmap"
	"github.com/MemeLabs/strims/pkg/ioutil"
	"github.com/MemeLabs/strims/pkg/ppspp/codec"
	"github.com/MemeLabs/strims/pkg/timeutil"
)

// ProtectionMethod ...
type ProtectionMethod uint8

// ProtectionMethods ...
const (
	_ ProtectionMethod = iota
	ProtectionMethodNone
	ProtectionMethodMerkleTree
	ProtectionMethodSignAll
)

// MerkleHashTreeFunction ...
type MerkleHashTreeFunction uint8

// MerkleHashTreeFunctions ...
const (
	_ MerkleHashTreeFunction = iota
	MerkleHashTreeFunctionSHA1
	MerkleHashTreeFunctionSHA256
	MerkleHashTreeFunctionSHA512
	MerkleHashTreeFunctionBLAKE2B256
	MerkleHashTreeFunctionBLAKE2B512
	MerkleHashTreeFunctionMD5
	MerkleHashTreeFunctionBLAKE3
	MerkleHashTreeFunctionSHA3224
	MerkleHashTreeFunctionSHA3256
	MerkleHashTreeFunctionSHA3384
	MerkleHashTreeFunctionSHA3512
)

// HashSize ...
func (f MerkleHashTreeFunction) HashSize() int {
	switch f {
	case MerkleHashTreeFunctionSHA1:
		return sha1.Size
	case MerkleHashTreeFunctionSHA256:
		return sha256.Size
	case MerkleHashTreeFunctionSHA512:
		return sha512.Size
	case MerkleHashTreeFunctionBLAKE2B256:
		return blake2b.Size256
	case MerkleHashTreeFunctionBLAKE2B512:
		return blake2b.Size
	case MerkleHashTreeFunctionBLAKE3:
		return blake3.New().Size()
	case MerkleHashTreeFunctionSHA3224:
		return sha3.New224().Size()
	case MerkleHashTreeFunctionSHA3256:
		return sha3.New256().Size()
	case MerkleHashTreeFunctionSHA3384:
		return sha3.New384().Size()
	case MerkleHashTreeFunctionSHA3512:
		return sha3.New512().Size()
	default:
		panic("unsupported hash tree function")
	}
}

// HashFunc ...
func (f MerkleHashTreeFunction) HashFunc() HashFunc {
	switch f {
	case MerkleHashTreeFunctionSHA1:
		return sha1.New
	case MerkleHashTreeFunctionSHA256:
		return sha256.New
	case MerkleHashTreeFunctionSHA512:
		return sha512.New
	case MerkleHashTreeFunctionBLAKE2B256:
		return blake2bFunc(blake2b.New256)
	case MerkleHashTreeFunctionBLAKE2B512:
		return blake2bFunc(blake2b.New512)
	case MerkleHashTreeFunctionBLAKE3:
		return blake3Func
	case MerkleHashTreeFunctionSHA3224:
		return sha3.New224
	case MerkleHashTreeFunctionSHA3256:
		return sha3.New256
	case MerkleHashTreeFunctionSHA3384:
		return sha3.New384
	case MerkleHashTreeFunctionSHA3512:
		return sha3.New512
	default:
		panic("unsupported hash tree function")
	}
}

// HashFunc ...
type HashFunc func() hash.Hash

func blake2bFunc(fn func([]byte) (hash.Hash, error)) HashFunc {
	return func() hash.Hash {
		h, _ := fn(nil)
		return h
	}
}

func blake3Func() hash.Hash {
	return blake3.New()
}

// LiveSignatureAlgorithm ...
type LiveSignatureAlgorithm uint8

// LiveSignatureAlgorithms ...
const (
	_ LiveSignatureAlgorithm = iota
	LiveSignatureAlgorithmED25519
)

// SignatureSize ...
func (a LiveSignatureAlgorithm) SignatureSize() int {
	switch a {
	case LiveSignatureAlgorithmED25519:
		return ed25519.SignatureSize
	default:
		panic("unsupported live signature algorithm")
	}
}

// Verifier ...
func (a LiveSignatureAlgorithm) Verifier(key []byte) SignatureVerifier {
	switch a {
	case LiveSignatureAlgorithmED25519:
		return NewED25519Verifier(key)
	default:
		panic("unsupported live signature algorithm")
	}
}

// NewDefaultVerifierOptions ...
func NewDefaultVerifierOptions() VerifierOptions {
	return VerifierOptions{
		ProtectionMethod:       ProtectionMethodMerkleTree,
		MerkleHashTreeFunction: MerkleHashTreeFunctionBLAKE2B256,
		LiveSignatureAlgorithm: LiveSignatureAlgorithmED25519,
	}
}

// SwarmVerifierOptions ...
type SwarmVerifierOptions struct {
	LiveDiscardWindow  int
	ChunkSize          int
	ChunksPerSignature int
	VerifierOptions
}

// MaxMessageBytes ...
func (o SwarmVerifierOptions) MaxMessageBytes() int {
	var im codec.Integrity
	var sim codec.SignedIntegrity

	switch o.ProtectionMethod {
	case ProtectionMethodSignAll:
		return codec.MessageTypeLen + o.LiveSignatureAlgorithm.SignatureSize() + sim.ByteLen()
	case ProtectionMethodMerkleTree:
		return codec.MessageTypeLen + o.LiveSignatureAlgorithm.SignatureSize() + sim.ByteLen() + bits.TrailingZeros64(uint64(o.ChunksPerSignature))*(codec.MessageTypeLen+o.MerkleHashTreeFunction.HashSize()+im.ByteLen())
	default:
		return 0
	}
}

// VerifierOptions ...
type VerifierOptions struct {
	ProtectionMethod       ProtectionMethod
	MerkleHashTreeFunction MerkleHashTreeFunction
	LiveSignatureAlgorithm LiveSignatureAlgorithm
}

// Assign ...
func (o *VerifierOptions) Assign(u VerifierOptions) {
	if u.ProtectionMethod != 0 {
		o.ProtectionMethod = u.ProtectionMethod
	}
	if u.MerkleHashTreeFunction != 0 {
		o.MerkleHashTreeFunction = u.MerkleHashTreeFunction
	}
	if u.LiveSignatureAlgorithm != 0 {
		o.LiveSignatureAlgorithm = u.LiveSignatureAlgorithm
	}
}

// Writer ...
type Writer interface {
	WriteSignedIntegrity(m codec.SignedIntegrity) (int, error)
	WriteIntegrity(m codec.Integrity) (int, error)
}

// NewVerifier ...
func NewVerifier(key []byte, opt SwarmVerifierOptions) (SwarmVerifier, error) {
	signatureVerifier := opt.LiveSignatureAlgorithm.Verifier(key)

	switch opt.ProtectionMethod {
	case ProtectionMethodNone:
		return &NoneSwarmVerifier{}, nil
	case ProtectionMethodMerkleTree:
		return NewMerkleSwarmVerifier(&MerkleOptions{
			LiveDiscardWindow:  opt.LiveDiscardWindow,
			ChunkSize:          opt.ChunkSize,
			ChunksPerSignature: opt.ChunksPerSignature,
			Verifier:           signatureVerifier,
			Hash:               opt.MerkleHashTreeFunction.HashFunc(),
		}), nil
	case ProtectionMethodSignAll:
		return NewSignAllSwarmVerifier(&SignAllOptions{
			LiveDiscardWindow: opt.LiveDiscardWindow,
			ChunkSize:         opt.ChunkSize,
			Verifier:          signatureVerifier,
		}), nil
	default:
		return nil, errors.New("unsupported protection method")
	}
}

// SwarmWriterOptions ...
type SwarmWriterOptions struct {
	LiveSignatureAlgorithm LiveSignatureAlgorithm
	ProtectionMethod       ProtectionMethod
	ChunkSize              int
	WriterOptions
}

// WriterOptions ...
type WriterOptions struct {
	ChunksPerSignature int
}

// NewWriter ...
func NewWriter(key []byte, v SwarmVerifier, w ioutil.WriteFlusher, opt SwarmWriterOptions) (ioutil.WriteFlusher, error) {
	var signatureSigner SignatureSigner
	switch opt.LiveSignatureAlgorithm {
	case LiveSignatureAlgorithmED25519:
		signatureSigner = NewED25519Signer(key)
	}

	switch opt.ProtectionMethod {
	case ProtectionMethodNone:
		return w, nil
	case ProtectionMethodMerkleTree:
		return NewMerkleWriter(&MerkleWriterOptions{
			ChunksPerSignature: opt.ChunksPerSignature,
			ChunkSize:          opt.ChunkSize,
			Verifier:           v.(*MerkleSwarmVerifier),
			Signer:             signatureSigner,
			Writer:             w,
		}), nil
	case ProtectionMethodSignAll:
		return NewSignAllWriter(&SignAllWriterOptions{
			ChunkSize: opt.ChunkSize,
			Verifier:  v.(*SignAllSwarmVerifier),
			Signer:    signatureSigner,
			Writer:    w,
		}), nil
	default:
		return nil, errors.New("unsupported protection method")
	}
}

// SwarmVerifier ...
type SwarmVerifier interface {
	WriteIntegrity(b binmap.Bin, m *binmap.Map, w Writer) (int, error)
	ChannelVerifier() ChannelVerifier
	ImportCache(c *swarmpb.Cache) error
	ExportCache() *swarmpb.Cache_Integrity
}

// ChannelVerifier ...
type ChannelVerifier interface {
	ChunkVerifier(b binmap.Bin) ChunkVerifier
}

// ChunkVerifier ...
type ChunkVerifier interface {
	SetSignedIntegrity(b binmap.Bin, t timeutil.Time, sig []byte)
	SetIntegrity(b binmap.Bin, hash []byte)
	Verify(b binmap.Bin, d []byte) (bool, error)
}

// SignatureSigner ...
type SignatureSigner interface {
	Sign(timestamp timeutil.Time, hash []byte) []byte
	Size() int
}

// SignatureVerifier ...
type SignatureVerifier interface {
	Verify(timestamp timeutil.Time, hash, sig []byte) bool
	Size() int
}
