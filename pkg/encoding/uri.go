package encoding

import (
	"errors"
	"net/url"
	"strings"
)

// TODO: implement this with content integrity...

// errors ...
var (
	ErrInvalidURIScheme = errors.New("invalid uri scheme")
)

var protocolOptionToURIKey = map[ProtocolOptionType]string{
	ContentIntegrityProtectionMethodOption: "x.im",
	MerkleHashTreeFunctionOption:           "x.hf",
	LiveSignatureAlgorithmOption:           "x.sa",
	ChunkAddressingMethodOption:            "x.am",
	ChunkSizeOption:                        "x.cs",
}

var uriScheme = "magnet:"

// NewURI ...
func NewURI() *URI {
	return &URI{
		options: map[ProtocolOptionType]uint8{},
	}
}

// URI ...
type URI struct {
	id      SwarmID
	options map[ProtocolOptionType]uint8
}

// String ...
func (u *URI) String() string {
	return ""
}

// ParseURI ...
func ParseURI(s string) (u *URI, err error) {
	if !strings.HasPrefix(s, uriScheme) {
		return nil, ErrInvalidURIScheme
	}

	query, err := url.ParseQuery(s[len(uriScheme):])
	if err != nil {
		return
	}

	u = NewURI()

	_ = query

	// for t, key := range protocolOptionToURIKey {
	// 	if query[key]
	// }

	return
}
