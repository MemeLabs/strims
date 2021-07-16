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
	IDLength    = 32
	idBitLength = IDLength * 8
)

// ID ...
type ID [4]uint64

// NewID ...
func NewID() (ID, error) {
	b := make([]byte, IDLength)
	_, err := rand.Read(b)
	if err != nil {
		return ID{}, err
	}
	return UnmarshalID(b)
}

// UnmarshalID ...
func UnmarshalID(b []byte) (d ID, err error) {
	_, err = d.Unmarshal(b)
	return
}

// MustUnmarshalID ....
func MustUnmarshalID(b []byte) ID {
	id, err := UnmarshalID(b)
	if err != nil {
		panic(err)
	}
	return id
}

// Unmarshal ...
func (d *ID) Unmarshal(b []byte) (int, error) {
	if len(b) < IDLength {
		return 0, errors.New("buffer too short")
	}

	d[0] = binary.BigEndian.Uint64(b)
	d[1] = binary.BigEndian.Uint64(b[8:])
	d[2] = binary.BigEndian.Uint64(b[16:])
	d[3] = binary.BigEndian.Uint64(b[24:])

	return IDLength, nil
}

// Marshal ...
func (d ID) Marshal(b []byte) (int, error) {
	if len(b) < IDLength {
		return 0, errors.New("buffer too short")
	}

	binary.BigEndian.PutUint64(b, d[0])
	binary.BigEndian.PutUint64(b[8:], d[1])
	binary.BigEndian.PutUint64(b[16:], d[2])
	binary.BigEndian.PutUint64(b[24:], d[3])
	return IDLength, nil
}

// MarshalJSON ...
func (d ID) MarshalJSON() ([]byte, error) {
	var b [IDLength]byte
	d.Bytes(b[:])
	j := make([]byte, base64.StdEncoding.EncodedLen(IDLength+2))
	base64.StdEncoding.Encode(j[1:], b[:])

	j[0] = '"'
	j = bytes.TrimRightFunc(j, func(r rune) bool { return r == 0 })
	j = append(j, '"')

	return j, nil
}

// UnmarshalJSON ...
func (d *ID) UnmarshalJSON(j []byte) error {
	var b [IDLength]byte
	_, err := base64.StdEncoding.Decode(b[:], j)
	if err != nil {
		return err
	}

	nd, err := UnmarshalID(b[:])
	if err != nil {
		return err
	}

	*d = nd
	return nil
}

// Bytes ...
func (d ID) Bytes(b []byte) []byte {
	if b == nil || len(b) < IDLength {
		b = make([]byte, IDLength)
	}
	if _, err := d.Marshal(b); err != nil {
		panic(err)
	}
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
	return d[0] == o[0] && d[1] == o[1] && d[2] == o[2] && d[3] == o[3]
}

// XOr ...
func (d ID) XOr(o ID) ID {
	return ID{d[0] ^ o[0], d[1] ^ o[1], d[2] ^ o[2], d[3] ^ o[3]}
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
