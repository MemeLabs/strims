package integrity

import (
	"crypto/ed25519"
	"crypto/sha1"
	"crypto/sha256"
	"crypto/sha512"
	"errors"
	"hash"
	"time"

	"golang.org/x/crypto/blake2b"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
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
	default:
		panic("unsupported hash tree function")
	}
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
	var signatureVerifier SignatureVerifier
	switch opt.LiveSignatureAlgorithm {
	case LiveSignatureAlgorithmED25519:
		signatureVerifier = NewED25519Verifier(key)
	}

	var hash hashFunc
	switch opt.MerkleHashTreeFunction {
	case MerkleHashTreeFunctionSHA1:
		hash = sha1.New
	case MerkleHashTreeFunctionSHA256:
		hash = sha256.New
	case MerkleHashTreeFunctionSHA512:
		hash = sha512.New
	case MerkleHashTreeFunctionBLAKE2B256:
		hash = blake2bFunc(blake2b.New256)
	case MerkleHashTreeFunctionBLAKE2B512:
		hash = blake2bFunc(blake2b.New512)
	}

	switch opt.ProtectionMethod {
	case ProtectionMethodNone:
		return &NoneSwarmVerifier{}, nil
	case ProtectionMethodMerkleTree:
		return NewMerkleSwarmVerifier(&MerkleOptions{
			LiveDiscardWindow:  opt.LiveDiscardWindow,
			ChunkSize:          opt.ChunkSize,
			ChunksPerSignature: opt.ChunksPerSignature,
			Verifier:           signatureVerifier,
			Hash:               hash,
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

type hashFunc func() hash.Hash

func blake2bFunc(fn func([]byte) (hash.Hash, error)) hashFunc {
	return func() hash.Hash {
		h, _ := fn(nil)
		return h
	}
}

// SwarmWriterOptions ...
type SwarmWriterOptions struct {
	LiveSignatureAlgorithm LiveSignatureAlgorithm
	ProtectionMethod       ProtectionMethod
	ChunkSize              int
	Verifier               SwarmVerifier
	Writer                 WriteFlusher
	WriterOptions
}

// WriterOptions ...
type WriterOptions struct {
	ChunksPerSignature int
}

// NewWriter ...
func NewWriter(key []byte, opt SwarmWriterOptions) (WriteFlusher, error) {
	var signatureSigner SignatureSigner
	switch opt.LiveSignatureAlgorithm {
	case LiveSignatureAlgorithmED25519:
		signatureSigner = NewED25519Signer(key)
	}

	switch opt.ProtectionMethod {
	case ProtectionMethodNone:
		return opt.Writer, nil
	case ProtectionMethodMerkleTree:
		return NewMerkleWriter(&MerkleWriterOptions{
			ChunksPerSignature: opt.ChunksPerSignature,
			ChunkSize:          opt.ChunkSize,
			Verifier:           opt.Verifier.(*MerkleSwarmVerifier),
			Signer:             signatureSigner,
			Writer:             opt.Writer,
		}), nil
	case ProtectionMethodSignAll:
		return NewSignAllWriter(&SignAllWriterOptions{
			ChunkSize: opt.ChunkSize,
			Verifier:  opt.Verifier.(*SignAllSwarmVerifier),
			Signer:    signatureSigner,
			Writer:    opt.Writer,
		}), nil
	default:
		return nil, errors.New("unsupported protection method")
	}
}

// SwarmVerifier ...
type SwarmVerifier interface {
	WriteIntegrity(b binmap.Bin, m *binmap.Map, w Writer) (int, error)
	ChannelVerifier() ChannelVerifier
}

// ChannelVerifier ...
type ChannelVerifier interface {
	ChunkVerifier(b binmap.Bin) ChunkVerifier
}

// ChunkVerifier ...
type ChunkVerifier interface {
	SetSignedIntegrity(b binmap.Bin, t time.Time, sig []byte)
	SetIntegrity(b binmap.Bin, hash []byte)
	Verify(b binmap.Bin, d []byte) bool
}

// SignatureSigner ...
type SignatureSigner interface {
	Sign(timestamp time.Time, hash []byte) []byte
	Size() int
}

// SignatureVerifier ...
type SignatureVerifier interface {
	Verify(timestamp time.Time, hash, sig []byte) bool
	Size() int
}

// WriteFlusher ...
type WriteFlusher interface {
	Write(p []byte) (int, error)
	Flush() error
}
