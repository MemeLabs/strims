package protoutil

import (
	"encoding/binary"
	"errors"
	"fmt"
	"io"
	"math"

	"github.com/MemeLabs/go-ppspp/pkg/pool"
	"google.golang.org/protobuf/proto"
)

var errBufferTooSmall = errors.New("buffer too small")

// ReadStream ...
func ReadStream(r io.Reader, m proto.Message) error {
	var t [2]byte
	if _, err := io.ReadFull(r, t[:]); err != nil {
		return fmt.Errorf("header read failed: %w", err)
	}
	n := binary.BigEndian.Uint16(t[:])

	b := pool.Get(int(n))
	defer pool.Put(b)

	if _, err := io.ReadFull(r, *b); err != nil {
		return fmt.Errorf("data read failed: %w", err)
	}
	if err := proto.Unmarshal(*b, m); err != nil {
		return fmt.Errorf("proto unmarshal failed: %w", err)
	}
	return nil
}

// WriteStream ...
func WriteStream(w io.Writer, m proto.Message) error {
	mn := proto.Size(m)
	n := mn + 2
	if n > int(math.MaxUint16) {
		return errBufferTooSmall
	}

	b := pool.Get(n)
	defer pool.Put(b)

	binary.BigEndian.PutUint16(*b, uint16(mn))
	_, err := proto.MarshalOptions{}.MarshalAppend((*b)[2:2], m)
	if err != nil {
		return fmt.Errorf("proto marshal failed: %w", err)
	}

	if _, err = w.Write(*b); err != nil {
		return fmt.Errorf("data write failed: %w", err)
	}
	return nil
}
