package dht

import (
	"crypto/rand"
	"encoding/binary"
	"errors"
	"fmt"
	"strings"
)

// constants ...
const (
	IDBytes = 20
	IDBits  = IDBytes * 8
)

// ID ...
type ID [5]uint32

// NewID ...
func NewID() (ID, error) {
	b := make([]byte, IDBytes)
	_, err := rand.Read(b)
	if err != nil {
		return ID{}, err
	}
	return UnmarshalID(b)
}

// UnmarshalID ...
func UnmarshalID(b []byte) (ID, error) {
	if len(b) < IDBytes {
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
	if len(b) < IDBytes {
		return 0, errors.New("not enough bytes in id")
	}

	binary.BigEndian.PutUint32(b, d[0])
	binary.BigEndian.PutUint32(b[4:], d[1])
	binary.BigEndian.PutUint32(b[8:], d[2])
	binary.BigEndian.PutUint32(b[12:], d[3])
	binary.BigEndian.PutUint32(b[16:], d[4])
	return IDBytes, nil
}

func (d ID) String() string {
	s := strings.Builder{}
	for i := range d {
		if i != 0 {
			s.WriteRune(' ')
		}
		s.WriteString(fmt.Sprintf("%032b", d[i]))
	}
	return s.String()
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
