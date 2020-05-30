package kademlia

import (
	"bytes"
	"crypto/rand"
	"encoding/base64"
	"encoding/binary"
	"encoding/hex"
	"errors"
)

// constants ...
const (
	IDByteLength = 20
	IDBitLength  = IDByteLength * 8
)

// ID ...
type ID [5]uint32

// NewID ...
func NewID() (ID, error) {
	b := make([]byte, IDByteLength)
	_, err := rand.Read(b)
	if err != nil {
		return ID{}, err
	}
	return UnmarshalID(b)
}

// UnmarshalID ...
func UnmarshalID(b []byte) (ID, error) {
	if len(b) < IDByteLength {
		return ID{}, errors.New("not enough bytes in id")
	}

	var d ID
	d[0] = binary.BigEndian.Uint32(b)
	d[1] = binary.BigEndian.Uint32(b[4:])
	d[2] = binary.BigEndian.Uint32(b[8:])
	d[3] = binary.BigEndian.Uint32(b[12:])
	d[4] = binary.BigEndian.Uint32(b[16:])

	return d, nil
}

// Marshal ...
func (d ID) Marshal(b []byte) (int, error) {
	if len(b) < IDByteLength {
		return 0, errors.New("not enough bytes in id")
	}

	binary.BigEndian.PutUint32(b, d[0])
	binary.BigEndian.PutUint32(b[4:], d[1])
	binary.BigEndian.PutUint32(b[8:], d[2])
	binary.BigEndian.PutUint32(b[12:], d[3])
	binary.BigEndian.PutUint32(b[16:], d[4])
	return IDByteLength, nil
}

// MarshalJSON ...
func (d ID) MarshalJSON() ([]byte, error) {
	var b [IDByteLength]byte
	d.Bytes(b[:])
	j := make([]byte, base64.StdEncoding.EncodedLen(IDByteLength+2))
	base64.StdEncoding.Encode(j[1:], b[:])

	j[0] = '"'
	j = bytes.TrimRightFunc(j, func(r rune) bool { return r == 0 })
	j = append(j, '"')

	return j, nil
}

// UnmarshalJSON ...
func (d *ID) UnmarshalJSON(j []byte) (err error) {
	var b [IDByteLength]byte
	base64.StdEncoding.Decode(b[:], j)
	*d, err = UnmarshalID(b[:])
	return
}

// Bytes ...
func (d ID) Bytes(b []byte) []byte {
	if b == nil || len(b) < IDByteLength {
		b = make([]byte, IDByteLength)
	}
	d.Marshal(b)
	return b
}

// String ...
func (d ID) String() string {
	return hex.EncodeToString(d.Bytes(nil))
}

// Clone ...
func (d ID) Clone() (c ID) {
	copy(c[:], d[:])
	return
}

// Equals ...
func (d ID) Equals(o ID) bool {
	return d[0] == o[0] && d[1] == o[1] && d[2] == o[2] && d[3] == o[3] && d[4] == o[4]
}

// XOr ...
func (d ID) XOr(o ID) ID {
	return ID{d[0] ^ o[0], d[1] ^ o[1], d[2] ^ o[2], d[3] ^ o[3], d[4] ^ o[4]}
}

// Less ...
func (d ID) Less(o ID) bool {
	for i := range d {
		if d[i] != o[i] {
			return d[i] < o[i]
		}
	}
	return false
}
