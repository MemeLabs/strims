package vpn

import (
	"encoding/binary"
	"io"
	"math"

	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"google.golang.org/protobuf/proto"
)

func sendProto(network *Network, id kademlia.ID, port, srcPort uint16, msg proto.Message) error {
	b := frameBuffer(uint16(proto.Size(msg)))
	defer freeFrameBuffer(b)

	b, err := proto.MarshalOptions{}.MarshalAppend(b[:0], msg)
	if err != nil {
		return err
	}

	return network.Send(id, port, srcPort, b)
}

func ReadProtoStream(r io.Reader, m proto.Message) error {
	var t [2]byte
	if _, err := io.ReadFull(r, t[:]); err != nil {
		return err
	}
	n := binary.BigEndian.Uint16(t[:])

	b := frameBuffer(n)
	defer freeFrameBuffer(b)

	if _, err := io.ReadFull(r, b[:n]); err != nil {
		return err
	}
	return proto.Unmarshal(b[:n], m)
}

func WriteProtoStream(w io.Writer, m proto.Message) error {
	mn := proto.Size(m)
	n := mn + 2
	if n > int(math.MaxUint16) {
		return errBufferTooSmall
	}

	b := frameBuffer(uint16(n))
	defer freeFrameBuffer(b)

	binary.BigEndian.PutUint16(b, uint16(mn))
	_, err := proto.MarshalOptions{}.MarshalAppend(b[2:2], m)
	if err != nil {
		return err
	}

	_, err = w.Write(b)
	return err
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
// 	b := frameBuffer(math.MaxUint16)
// 	defer freeFrameBuffer(b)

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
