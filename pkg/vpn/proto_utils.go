package vpn

import (
	"crypto/ed25519"
	"encoding/binary"
	"errors"
	"io"
	"math"

	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/reflect/protoreflect"
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

func signProto(msg proto.Message, key *pb.Key) error {
	if key.Type != pb.KeyType_KEY_TYPE_ED25519 {
		return errors.New("unsupported key type")
	}

	r := msg.ProtoReflect()
	fields := r.Descriptor().Fields()
	keyField := fields.ByName("key")
	signatureField := fields.ByName("signature")

	r.Set(keyField, protoreflect.ValueOfBytes(key.Public))
	r.Set(signatureField, protoreflect.ValueOfBytes(nil))

	b := frameBuffer(uint16(proto.Size(msg)))
	defer freeFrameBuffer(b)

	b, err := proto.MarshalOptions{}.MarshalAppend(b[:0], msg)
	if err != nil {
		return err
	}

	signature := ed25519.Sign(ed25519.PrivateKey(key.Private), b)
	r.Set(signatureField, protoreflect.ValueOfBytes(signature))
	return nil
}

func verifyProto(msg proto.Message) bool {
	r := msg.ProtoReflect()
	fields := r.Descriptor().Fields()
	keyField := fields.ByName("key")
	signatureField := fields.ByName("signature")

	key := r.Get(keyField).Bytes()
	signature := r.Get(signatureField).Bytes()

	c := proto.Clone(msg)
	c.ProtoReflect().Set(signatureField, protoreflect.ValueOfBytes(nil))

	b := frameBuffer(uint16(proto.Size(c)))
	defer freeFrameBuffer(b)

	b, err := proto.MarshalOptions{}.MarshalAppend(b[:0], c)
	if err != nil {
		return false
	}

	if len(key) != ed25519.PublicKeySize {
		return false
	}
	return ed25519.Verify(ed25519.PublicKey(key), b, signature)
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
