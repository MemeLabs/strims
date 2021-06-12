package ppspp

import (
	"errors"
	"net/url"
	"strconv"
	"strings"

	"github.com/MemeLabs/go-ppspp/pkg/ppspp/codec"
	"github.com/MemeLabs/go-ppspp/pkg/ppspp/integrity"
)

// TODO: implement this with content integrity...

// errors ...
var (
	ErrInvalidURI = errors.New("invalid uri")
)

var protocolOptions = []struct {
	Type codec.ProtocolOptionType
	Key  string
}{
	{
		codec.ContentIntegrityProtectionMethodOption,
		"x.im",
	},
	{
		codec.MerkleHashTreeFunctionOption,
		"x.hf",
	},
	{
		codec.LiveSignatureAlgorithmOption,
		"x.sa",
	},
	{
		codec.ChunkSizeOption,
		"x.cs",
	},
	{
		codec.ChunksPerSignatureOption,
		"x.ps",
	},
	{
		codec.StreamCountOption,
		"x.sc",
	},
}

var uriScheme = "magnet:"
var urnPrefix = "urn:ppspp:"

// URIOptions ...
type URIOptions map[codec.ProtocolOptionType]int

// SwarmOptions ...
func (o URIOptions) SwarmOptions() SwarmOptions {
	return SwarmOptions{
		ChunkSize:          o[codec.ChunkSizeOption],
		ChunksPerSignature: o[codec.ChunksPerSignatureOption],
		StreamCount:        o[codec.StreamCountOption],
		Integrity: integrity.VerifierOptions{
			ProtectionMethod:       integrity.ProtectionMethod(o[codec.ContentIntegrityProtectionMethodOption]),
			MerkleHashTreeFunction: integrity.MerkleHashTreeFunction(o[codec.MerkleHashTreeFunctionOption]),
			LiveSignatureAlgorithm: integrity.LiveSignatureAlgorithm(o[codec.LiveSignatureAlgorithmOption]),
		},
	}
}

// NewURI ...
func NewURI(id SwarmID, options URIOptions) *URI {
	return &URI{
		ID:      id,
		Options: options,
	}
}

// URI ...
type URI struct {
	ID      SwarmID
	Options URIOptions
}

// String ...
func (u *URI) String() string {
	var s strings.Builder
	s.WriteString(uriScheme)
	s.WriteString("?xt=")
	s.WriteString(urnPrefix)
	s.WriteString(u.ID.String())

	for _, opt := range protocolOptions {
		v, ok := u.Options[opt.Type]
		if !ok {
			continue
		}
		s.WriteRune('&')
		s.WriteString(opt.Key)
		s.WriteRune('=')
		s.WriteString(strconv.FormatUint(uint64(v), 10))
	}

	return s.String()
}

// ParseURI ...
func ParseURI(s string) (u *URI, err error) {
	u = &URI{
		Options: URIOptions{},
	}

	parts := strings.SplitN(s, "?", 2)
	if len(parts) != 2 || parts[0] != uriScheme {
		return nil, ErrInvalidURI
	}

	query, err := url.ParseQuery(parts[1])
	if err != nil {
		return
	}

	xt := query.Get("xt")
	if !strings.HasPrefix(xt, urnPrefix) {
		return nil, ErrInvalidURI
	}
	u.ID, err = DecodeSwarmID(strings.TrimPrefix(xt, urnPrefix))
	if err != nil {
		return
	}

	for _, opt := range protocolOptions {
		vs, ok := query[opt.Key]
		if !ok {
			continue
		}
		v, err := strconv.ParseUint(vs[0], 10, 31)
		if err != nil {
			return nil, err
		}
		u.Options[opt.Type] = int(v)
	}

	return
}
