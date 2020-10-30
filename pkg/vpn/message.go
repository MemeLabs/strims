package vpn

import (
	"crypto/ed25519"
	"encoding/binary"
	"encoding/hex"
	"errors"
	"io"

	"github.com/MemeLabs/go-ppspp/pkg/kademlia"
	"github.com/MemeLabs/go-ppspp/pkg/pool"
	"github.com/MemeLabs/go-ppspp/pkg/vnic"
)

var errBufferTooSmall = errors.New("buffer too small")

const messageHeaderLen = kademlia.IDLength + 8

// MessageHeader ...
type MessageHeader struct {
	DstID   kademlia.ID
	DstPort uint16
	SrcPort uint16
	Seq     uint16
	Length  uint16
}

// Marshal ...
func (m MessageHeader) Marshal(b []byte) (n int, err error) {
	if len(b) < messageHeaderLen {
		return 0, errBufferTooSmall
	}
	if _, err = m.DstID.Marshal(b); err != nil {
		return
	}
	binary.BigEndian.PutUint16(b[kademlia.IDLength:], m.DstPort)
	binary.BigEndian.PutUint16(b[kademlia.IDLength+2:], m.SrcPort)
	binary.BigEndian.PutUint16(b[kademlia.IDLength+4:], m.Seq)
	binary.BigEndian.PutUint16(b[kademlia.IDLength+6:], m.Length)
	return messageHeaderLen, nil
}

// Unmarshal ...
func (m *MessageHeader) Unmarshal(b []byte) (n int, err error) {
	if len(b) < messageHeaderLen {
		return 0, errBufferTooSmall
	}
	m.DstID, err = kademlia.UnmarshalID(b)
	if err != nil {
		return
	}
	m.DstPort = binary.BigEndian.Uint16(b[kademlia.IDLength:])
	m.SrcPort = binary.BigEndian.Uint16(b[kademlia.IDLength+2:])
	m.Seq = binary.BigEndian.Uint16(b[kademlia.IDLength+4:])
	m.Length = binary.BigEndian.Uint16(b[kademlia.IDLength+6:])
	return messageHeaderLen, nil
}

const messageTrailerLen = kademlia.IDLength + 64

// Trailers represents the messages path
// index 0 is the sender and subsequent indexes are hops
type Trailers []MessageTrailer

// MessageTrailer represents a node in the message path
type MessageTrailer struct {
	HostID    kademlia.ID
	Signature []byte
}

// Marshal ...
func (m *MessageTrailer) Marshal(b []byte) (n int, err error) {
	if len(b) < messageTrailerLen {
		return 0, errBufferTooSmall
	}
	if _, err = m.HostID.Marshal(b); err != nil {
		return
	}
	copy(b[kademlia.IDLength:], m.Signature)
	return messageTrailerLen, nil
}

// Unmarshal ...
func (m *MessageTrailer) Unmarshal(b []byte) (n int, err error) {
	if len(b) < messageTrailerLen {
		return 0, errBufferTooSmall
	}
	m.HostID, err = kademlia.UnmarshalID(b)
	if err != nil {
		return
	}
	m.Signature = b[kademlia.IDLength:]
	return messageTrailerLen, nil
}

// Contains ...
func (t Trailers) Contains(hostID kademlia.ID) bool {
	for i := range t {
		if t[i].HostID.Equals(hostID) {
			return true
		}
	}
	return false
}

// Message ...
type Message struct {
	Header   MessageHeader
	Body     []byte
	Trailers Trailers
}

// MessageID ...
type MessageID [2 + kademlia.IDLength]byte

// String ...
func (m MessageID) String() string {
	return hex.EncodeToString(m[:])
}

// ID ...
func (m *Message) ID() (id MessageID) {
	binary.BigEndian.PutUint16(id[:2], m.Header.Seq)
	m.FromHostID().Bytes(id[2:])
	return
}

// Size ...
func (m *Message) Size() int {
	return messageHeaderLen + len(m.Body) + (m.Hops()+1)*messageTrailerLen
}

// Hops ...
func (m *Message) Hops() int {
	return len(m.Trailers)
}

// FromHostID ...
func (m *Message) FromHostID() (id kademlia.ID) {
	if len(m.Trailers) == 0 {
		return
	}
	return m.Trailers[0].HostID
}

// Marshal ...
func (m *Message) Marshal(b []byte, host *vnic.Host) (n int, err error) {
	if len(b) < m.Size() {
		return 0, errBufferTooSmall
	}

	d, err := m.Header.Marshal(b)
	if err != nil {
		return
	}
	n += d

	n += copy(b[n:], m.Body)

	for i := 0; i < m.Hops(); i++ {
		d, err = m.Trailers[i].Marshal(b[n:])
		if err != nil {
			return
		}
		n += d
	}

	d, err = host.ID().Marshal(b[n:])
	if err != nil {
		return
	}
	n += d
	n += copy(b[n:], ed25519.Sign(host.Key().Private, b[:n]))

	return n, nil
}

// Unmarshal ...
func (m *Message) Unmarshal(b []byte) (n int, err error) {
	d, err := m.Header.Unmarshal(b)
	if err != nil {
		return
	}
	n = d

	d += int(m.Header.Length)
	if len(b) < d {
		return 0, errBufferTooSmall
	}
	m.Body = b[n:d]
	n = d

	m.Trailers = make([]MessageTrailer, (len(b)-n)/messageTrailerLen)
	for i := 0; i < len(m.Trailers); i++ {
		d, err = m.Trailers[i].Unmarshal(b[n : n+messageTrailerLen])
		if err != nil {
			return 0, err
		}
		n += d
	}

	return n, nil
}

// WriteTo ...
func (m Message) WriteTo(w io.Writer, host *vnic.Host) (int64, error) {
	b := pool.Get(uint16(m.Size()))
	defer pool.Put(b)

	n, err := m.Marshal(*b, host)
	if err != nil {
		return 0, err
	}
	if _, err = w.Write((*b)[:n]); err != nil {
		return 0, err
	}
	return int64(n), nil
}

// func (m *Message) ReadFrom(r io.Reader) (int64, error) {
// 	hn, err := m.Header.ReadFrom(r)
// 	if err != nil {
// 		return 0, err
// 	}
// 	f.Body = bufferPool.Get(f.Header.Length)[:f.Header.Length]
// 	bn, err := io.ReadFull(r, f.Body)
// 	return hn + int64(bn), err
// }
