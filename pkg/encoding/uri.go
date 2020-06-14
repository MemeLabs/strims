package encoding

import (
	"errors"
	"net/url"
	"strconv"
	"strings"
)

// TODO: implement this with content integrity...

// errors ...
var (
	ErrInvalidURI = errors.New("invalid uri")
)

var protocolOptions = []struct {
	Type ProtocolOptionType
	Key  string
}{
	{
		ContentIntegrityProtectionMethodOption,
		"x.im",
	},
	{
		MerkleHashTreeFunctionOption,
		"x.hf",
	},
	{
		LiveSignatureAlgorithmOption,
		"x.sa",
	},
	{
		ChunkAddressingMethodOption,
		"x.am",
	},
	{
		ChunkSizeOption,
		"x.cs",
	},
}

var uriScheme = "magnet:"
var urnPrefix = "urn:ppspp:"

// URIOptions ...
type URIOptions map[ProtocolOptionType]int

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
		v, err := strconv.ParseUint(vs[0], 10, 8)
		if err != nil {
			return nil, err
		}
		u.Options[opt.Type] = int(v)
	}

	return
}
