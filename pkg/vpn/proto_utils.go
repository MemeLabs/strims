package vpn

import (
	"encoding/binary"
	"fmt"
	"io"
	"math"

	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/pool"
	"google.golang.org/protobuf/proto"
)

func sendProto(network *Network, id kademlia.ID, port, srcPort uint16, msg proto.Message) error {
	b := pool.Get(uint16(proto.Size(msg)))
	defer pool.Put(b)

	_, err := proto.MarshalOptions{}.MarshalAppend((*b)[:0], msg)
	if err != nil {
		return err
	}

	return network.Send(id, port, srcPort, *b)
}

// ReadProtoStream ...
func ReadProtoStream(r io.Reader, m proto.Message) error {
	var t [2]byte
	if _, err := io.ReadFull(r, t[:]); err != nil {
		return fmt.Errorf("header read failed: %w", err)
	}
	n := binary.BigEndian.Uint16(t[:])

	b := pool.Get(n)
	defer pool.Put(b)

	if _, err := io.ReadFull(r, *b); err != nil {
		return fmt.Errorf("data read failed: %w", err)
	}
	if err := proto.Unmarshal(*b, m); err != nil {
		return fmt.Errorf("proto unmarshal failed: %w", err)
	}
	return nil
}

// WriteProtoStream ...
func WriteProtoStream(w io.Writer, m proto.Message) error {
	mn := proto.Size(m)
	n := mn + 2
	if n > int(math.MaxUint16) {
		return errBufferTooSmall
	}

	b := pool.Get(uint16(n))
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

// func readProtoFrame(r io.Reader, m proto.Message) error {
// 	var f frame
// 	defer f.Free()

// 	if _, err := f.ReadFrom(r); err != nil {
// 		return err
// 	}
// 	return proto.Unmarshal(f.Body, m)
// }

// func writeProtoFrame(w io.Writer, port uint16, m proto.Message) error {
// 	b := bufferPool.Get(math.MaxUint16)
// 	defer bufferPool.Put(b)

// 	b, err := proto.MarshalOptions{}.MarshalAppend(b[:0], m)
// 	if err != nil {
// 		return err
// 	}
// 	f := frame{
// 		Header: frameHeader{
// 			Port:   port,
// 			Length: uint16(len(b)),
// 		},
// 		Body: b,
// 	}
// 	_, err = f.WriteTo(w)
// 	return err
// }
