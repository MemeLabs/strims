package encoding

import (
	"encoding/binary"
	"errors"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/binmap"
)

// errors ...
var (
	ErrUnsupportedMessageType    = errors.New("unsupported message type")
	ErrUnsupportedProtocolOption = errors.New("unsupported protocol option")
)

// Reader ...
type Reader interface {
	Unmarshal(b []byte) (int, error)
}

// Writer ...
type Writer interface {
	Marshal(b []byte) int
}

// Message ...
type Message interface {
	Reader
	Writer
	Type() MessageType
	ByteLen() int
}

// Address ...
type Address binmap.Bin

// NewAddress ...
func NewAddress(b binmap.Bin) *Address {
	a := Address(b)
	return &a
}

// Unmarshal ...
func (v *Address) Unmarshal(b []byte) (int, error) {
	*v = Address(binary.BigEndian.Uint64(b))
	return 8, nil
}

// Marshal ...
func (v *Address) Marshal(b []byte) int {
	binary.BigEndian.PutUint64(b, uint64(v.Bin()))
	return 8
}

// ByteLen ...
func (v *Address) ByteLen() int {
	return 8
}

// DataLen ...
func (v *Address) DataLen() uint64 {
	return uint64(v.Bin().BaseLength()) * uint64(ChunkSize)
}

// DataOffset ...
func (v *Address) DataOffset() uint64 {
	return uint64(v.Bin().BaseOffset()) * uint64(ChunkSize)
}

// Bin ...
func (v *Address) Bin() binmap.Bin {
	return binmap.Bin(*v)
}

// Buffer ...
type Buffer []byte

// Unmarshal ...
func (v *Buffer) Unmarshal(b []byte) (int, error) {
	*v = Buffer(b)
	return len(b), nil
}

// Marshal ...
func (v Buffer) Marshal(b []byte) int {
	return copy(b, v)
}

// ByteLen ...
func (v Buffer) ByteLen() int {
	return len(v)
}

// ProtocolOption ...
type ProtocolOption interface {
	Reader
	Writer
	Type() ProtocolOptionType
	ByteLen() int
}

// ProtocolOptions ...
type ProtocolOptions []ProtocolOption

// Find ...
func (o ProtocolOptions) Find(t ProtocolOptionType) (ProtocolOption, bool) {
	for _, opt := range o {
		if opt.Type() == t {
			return opt, true
		}
	}
	return nil, false
}

// VersionProtocolOption ...
type VersionProtocolOption struct {
	Value uint8
}

// Unmarshal ...
func (v *VersionProtocolOption) Unmarshal(b []byte) (int, error) {
	v.Value = b[0]
	return 1, nil
}

// Marshal ...
func (v *VersionProtocolOption) Marshal(b []byte) int {
	b[0] = v.Value
	return 1
}

// Type ...
func (v *VersionProtocolOption) Type() ProtocolOptionType {
	return VersionOption
}

// ByteLen ...
func (v *VersionProtocolOption) ByteLen() int {
	return 1
}

// MinimumVersionProtocolOption ...
type MinimumVersionProtocolOption struct {
	Value uint8
}

// Unmarshal ...
func (v *MinimumVersionProtocolOption) Unmarshal(b []byte) (int, error) {
	v.Value = b[0]
	return 1, nil
}

// Marshal ...
func (v *MinimumVersionProtocolOption) Marshal(b []byte) int {
	b[0] = v.Value
	return 1
}

// Type ...
func (v *MinimumVersionProtocolOption) Type() ProtocolOptionType {
	return MinimumVersionOption
}

// ByteLen ...
func (v *MinimumVersionProtocolOption) ByteLen() int {
	return 1
}

// NewSwarmIdentifierProtocolOption ...
func NewSwarmIdentifierProtocolOption() *SwarmIdentifierProtocolOption {
	return &SwarmIdentifierProtocolOption{}
}

// SwarmIdentifierProtocolOption ...
type SwarmIdentifierProtocolOption []byte

// Unmarshal ...
func (v *SwarmIdentifierProtocolOption) Unmarshal(b []byte) (size int, err error) {
	idSize := int(binary.BigEndian.Uint16(b))
	size += 2

	*v = b[2 : size+idSize]
	size += idSize

	return
}

// Marshal ...
func (v *SwarmIdentifierProtocolOption) Marshal(b []byte) (size int) {
	binary.BigEndian.PutUint16(b, uint16(len(*v)))
	size += 2

	size += copy(b[2:], *v)

	return
}

// Type ...
func (v *SwarmIdentifierProtocolOption) Type() ProtocolOptionType {
	return SwarmIdentifierOption
}

// ByteLen ...
func (v *SwarmIdentifierProtocolOption) ByteLen() int {
	return 2 + len(*v)
}

// Handshake ...
type Handshake struct {
	ChannelID uint32
	Options   ProtocolOptions
}

// NewHandshake ...
func NewHandshake(channelID uint32) *Handshake {
	return &Handshake{
		ChannelID: channelID,
		Options:   ProtocolOptions{},
	}
}

// Unmarshal ...
func (v *Handshake) Unmarshal(b []byte) (size int, err error) {
	v.ChannelID = binary.BigEndian.Uint32(b)
	size += 4

EachProtocolOption:
	for size < len(b) {
		optionType := ProtocolOptionType(b[size])
		size++

		var option ProtocolOption
		switch optionType {
		case VersionOption:
			option = new(VersionProtocolOption)
		case MinimumVersionOption:
			option = new(MinimumVersionProtocolOption)
		case SwarmIdentifierOption:
			option = new(SwarmIdentifierProtocolOption)
		case EndOption:
			break EachProtocolOption
		default:
			return 0, ErrUnsupportedProtocolOption
		}

		var optionSize int
		optionSize, err = option.Unmarshal(b[size:])
		size += optionSize

		v.Options = append(v.Options, option)
	}

	return
}

// Marshal ...
func (v *Handshake) Marshal(b []byte) (size int) {
	binary.BigEndian.PutUint32(b, v.ChannelID)
	size += 4

	for _, option := range v.Options {
		b[size] = byte(option.Type())
		size++

		size += option.Marshal(b[size:])
	}

	return
}

// Type ...
func (v *Handshake) Type() MessageType {
	return HandshakeMessage
}

// ByteLen ...
func (v *Handshake) ByteLen() (l int) {
	for _, option := range v.Options {
		l += option.ByteLen() + 1
	}
	return l + 5
}

// Data ...
type Data struct {
	Address   Address
	Timestamp Timestamp
	Data      Buffer
}

// NewData ...
func NewData(b binmap.Bin, d []byte) *Data {
	return &Data{
		Address:   Address(b),
		Timestamp: Timestamp{time.Now()},
		Data:      Buffer(d),
	}
}

// Type ...
func (v *Data) Type() MessageType {
	return DataMessage
}

// Unmarshal ...
func (v *Data) Unmarshal(b []byte) (size int, err error) {
	n, err := v.Address.Unmarshal(b)
	if err != nil {
		return
	}
	size += n

	n, err = v.Timestamp.Unmarshal(b[size:])
	if err != nil {
		return
	}
	size += n

	n = int(v.Address.DataLen())
	if size+n > len(b) {
		n = len(b) - size
	}
	v.Data = b[size : size+n]
	size += n

	return
}

// Marshal ...
func (v *Data) Marshal(b []byte) (size int) {
	size += v.Address.Marshal(b)
	size += v.Timestamp.Marshal(b[size:])
	size += v.Data.Marshal(b[size:])

	return
}

// ByteLen ...
func (v *Data) ByteLen() int {
	return int(v.Address.ByteLen()) + v.Timestamp.ByteLen() + v.Data.ByteLen()
}

// Timestamp ...
type Timestamp struct {
	time.Time
}

// Unmarshal ...
func (v *Timestamp) Unmarshal(b []byte) (int, error) {
	v.Time = time.Unix(
		int64(binary.BigEndian.Uint32(b)),
		int64(binary.BigEndian.Uint32(b[4:])),
	)

	return 8, nil
}

// Marshal ...
func (v *Timestamp) Marshal(b []byte) (size int) {
	binary.BigEndian.PutUint32(b, uint32(v.Time.Unix()))
	binary.BigEndian.PutUint32(b[4:], uint32(v.Time.Nanosecond()))

	return 8
}

// ByteLen ...
func (v *Timestamp) ByteLen() int {
	return 8
}

// Ack ...
type Ack struct {
	Address     Address
	DelaySample Timestamp
}

// Type ...
func (v *Ack) Type() MessageType {
	return AckMessage
}

// Unmarshal ...
func (v *Ack) Unmarshal(b []byte) (size int, err error) {
	n, err := v.Address.Unmarshal(b)
	if err != nil {
		return
	}
	size += n

	n, err = v.DelaySample.Unmarshal(b[n:])
	if err != nil {
		return
	}
	size += n

	return
}

// Marshal ...
func (v *Ack) Marshal(b []byte) (size int) {
	size += v.Address.Marshal(b)
	size += v.DelaySample.Marshal(b[size:])

	return
}

// ByteLen ...
func (v *Ack) ByteLen() int {
	return v.Address.ByteLen() + v.DelaySample.ByteLen()
}

// Nonce ...
type Nonce struct {
	Value uint64
}

// Unmarshal ...
func (v *Nonce) Unmarshal(b []byte) (size int, err error) {
	v.Value = binary.BigEndian.Uint64(b)
	size += 8

	return
}

// Marshal ...
func (v *Nonce) Marshal(b []byte) (size int) {
	binary.BigEndian.PutUint64(b, v.Value)
	size += 8

	return
}

// ByteLen ...
func (v *Nonce) ByteLen() int {
	return 8
}

// Ping ...
type Ping struct {
	Nonce
}

// Type ...
func (v *Ping) Type() MessageType {
	return PingMessage
}

// Pong ...
type Pong struct {
	Nonce
}

// Type ...
func (v *Pong) Type() MessageType {
	return PongMessage
}

// Have ...
type Have struct {
	Address
}

// Type ...
func (v *Have) Type() MessageType {
	return HaveMessage
}

// Request ...
type Request struct {
	Address
}

// Type ...
func (v *Request) Type() MessageType {
	return RequestMessage
}

// Cancel ...
type Cancel struct {
	Address
}

// Type ...
func (v *Cancel) Type() MessageType {
	return CancelMessage
}

// Empty ...
type Empty struct{}

// Unmarshal ...
func (v *Empty) Unmarshal(b []byte) (int, error) {
	return 0, nil
}

// Marshal ...
func (v *Empty) Marshal(b []byte) int {
	return 0
}

// ByteLen ...
func (v *Empty) ByteLen() int {
	return 0
}

// Choke ...
type Choke struct {
	Empty
}

// Type ...
func (v *Choke) Type() MessageType {
	return ChokeMessage
}

// Unchoke ...
type Unchoke struct {
	Empty
}

// Type ...
func (v *Unchoke) Type() MessageType {
	return UnchokeMessage
}

// End ...
type End struct {
	Empty
}

// Type ...
func (v *End) Type() MessageType {
	return EndMessage
}

// Messages ...
type Messages []Message

// Unmarshal ...
func (v *Messages) Unmarshal(b []byte) (n int, err error) {
	for {
		if n == len(b) {
			return
		}

		var msg Message
		mt := MessageType(b[n])
		n++

		switch mt {
		case HandshakeMessage:
			msg = &Handshake{}
		case DataMessage:
			msg = &Data{}
		case AckMessage:
			msg = &Ack{}
		case HaveMessage:
			msg = &Have{}
		case RequestMessage:
			msg = &Request{}
		case CancelMessage:
			msg = &Cancel{}
		case ChokeMessage:
			msg = &Choke{}
		case UnchokeMessage:
			msg = &Unchoke{}
		case PingMessage:
			msg = &Ping{}
		case PongMessage:
			msg = &Pong{}
		case EndMessage:
			return
		default:
			return n, ErrUnsupportedMessageType
		}

		mn, err := msg.Unmarshal(b[n:])
		if err != nil {
			return n, err
		}
		n += mn
		*v = append(*v, msg)
	}
}

// Marshal ...
func (v *Messages) Marshal(b []byte) (size int) {
	for _, m := range *v {
		b[0] = byte(m.Type())
		size++
		b = b[1:]

		n := m.Marshal(b)

		size += n
		b = b[n:]
	}

	return
}

// ByteLen ...
func (v *Messages) ByteLen() (l int) {
	for _, m := range *v {
		l += m.ByteLen() + 1
	}
	return
}

// Datagram ...
type Datagram struct {
	ChannelID uint32
	Messages  Messages
}

// Unmarshal ...
func (v *Datagram) Unmarshal(b []byte) (size int, err error) {
	v.ChannelID = binary.BigEndian.Uint32(b)
	size += 4

	n, err := v.Messages.Unmarshal(b[size:])
	size += n

	return
}

// Marshal ...
func (v *Datagram) Marshal(b []byte) (size int) {
	binary.BigEndian.PutUint32(b, v.ChannelID)
	size += 4

	n := v.Messages.Marshal(b[size:])
	size += n

	return
}

// ByteLen ...
func (v *Datagram) ByteLen() int {
	return 4 + v.Messages.ByteLen()
}
