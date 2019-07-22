package encoding

import (
	"encoding/binary"
	"errors"
	"time"
)

var ErrUnsupportedMessageType = errors.New("unsupported message type")
var ErrUnsupportedProtocolOption = errors.New("unsupported protocol option")

type Bin uint32

func (b Bin) BaseLeft() Bin {
	return b & (b + 1)
}

func (b Bin) BaseRight() Bin {
	return (b | (b + 1)) - 1
}

func (b Bin) BaseLen() int {
	t := b + 1
	return int(t & -t)
}

type Reader interface {
	Unmarshal(b []byte) (int, error)
}

type Writer interface {
	Marshal(b []byte) int
}

type Message interface {
	Reader
	Writer
	Type() MessageType
}

type Address interface {
	Reader
	Writer
	ByteLen() int
}

type Bin32ChunkAddress Bin

func NewBin32ChunkAddress(b Bin) *Bin32ChunkAddress {
	a := Bin32ChunkAddress(b)
	return &a
}

func (v *Bin32ChunkAddress) Unmarshal(b []byte) (int, error) {
	*v = Bin32ChunkAddress(binary.BigEndian.Uint32(b))
	return 4, nil
}

func (v *Bin32ChunkAddress) Marshal(b []byte) int {
	binary.BigEndian.PutUint32(b, uint32(*v))
	return 4
}

func (v *Bin32ChunkAddress) ByteLen() int {
	return Bin(*v).BaseLen() * ChunkSize
}

type Buffer []byte

func (v *Buffer) Unmarshal(b []byte) (int, error) {
	*v = Buffer(b)
	return len(b), nil
}

func (v Buffer) Marshal(b []byte) int {
	return copy(b, v)
}

type ProtocolOption interface {
	Reader
	Writer
	Type() ProtocolOptionType
}

type ProtocolOptions []ProtocolOption

func (o ProtocolOptions) Find(t ProtocolOptionType) (ProtocolOption, bool) {
	for _, opt := range o {
		if opt.Type() == t {
			return opt, true
		}
	}
	return nil, false
}

type VersionProtocolOption struct {
	Value uint8
}

func (v *VersionProtocolOption) Unmarshal(b []byte) (int, error) {
	v.Value = b[0]
	return 1, nil
}

func (v *VersionProtocolOption) Marshal(b []byte) int {
	b[0] = v.Value
	return 1
}

func (v *VersionProtocolOption) Type() ProtocolOptionType {
	return VersionOption
}

type MinimumVersionProtocolOption struct {
	Value uint8
}

func (v *MinimumVersionProtocolOption) Unmarshal(b []byte) (int, error) {
	v.Value = b[0]
	return 1, nil
}

func (v *MinimumVersionProtocolOption) Marshal(b []byte) int {
	b[0] = v.Value
	return 1
}

func (v *MinimumVersionProtocolOption) Type() ProtocolOptionType {
	return MinimumVersionOption
}

func NewSwarmIdentifierProtocolOption() *SwarmIdentifierProtocolOption {
	return &SwarmIdentifierProtocolOption{}
}

type SwarmIdentifierProtocolOption []byte

func (v *SwarmIdentifierProtocolOption) Unmarshal(b []byte) (size int, err error) {
	idSize := int(binary.BigEndian.Uint16(b))
	size += 2

	*v = b[2 : size+idSize]
	size += idSize

	return
}

func (v *SwarmIdentifierProtocolOption) Marshal(b []byte) (size int) {
	binary.BigEndian.PutUint16(b, uint16(len(*v)))
	size += 2

	size += copy(b[2:], *v)

	return
}

func (v *SwarmIdentifierProtocolOption) Type() ProtocolOptionType {
	return SwarmIdentifierOption
}

type Handshake struct {
	ChannelID uint32
	Options   ProtocolOptions
}

func NewHandshake(channelID uint32) *Handshake {
	return &Handshake{
		ChannelID: channelID,
		Options:   ProtocolOptions{},
	}
}

func (v *Handshake) Unmarshal(b []byte) (size int, err error) {
	v.ChannelID = binary.BigEndian.Uint32(b)
	size += 4

EachProtocolOption:
	for size < len(b) {
		optionType := ProtocolOptionType(b[size])
		size += 1

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

func (v *Handshake) Marshal(b []byte) (size int) {
	binary.BigEndian.PutUint32(b, v.ChannelID)
	size += 4

	for _, option := range v.Options {
		b[size] = byte(option.Type())
		size += 1

		size += option.Marshal(b[size:])
	}

	b[size] = byte(EndOption)
	size += 1

	return
}

func (v *Handshake) Type() MessageType {
	return HANDSHAKE
}

type Data struct {
	Address   Address
	Timestamp Timestamp
	Data      Buffer
}

func NewData(b Bin, d []byte) *Data {
	return &Data{
		Address:   NewBin32ChunkAddress(b),
		Timestamp: Timestamp{time.Now()},
		Data:      Buffer(d),
	}
}

func (v *Data) Type() MessageType {
	return DATA
}

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

	n = v.Address.ByteLen()
	if size+n > len(b) {
		n = len(b) - size
	}
	v.Data = b[size : size+n]
	size += n

	return
}

func (v *Data) Marshal(b []byte) (size int) {
	size += v.Address.Marshal(b)
	size += v.Timestamp.Marshal(b[size:])
	size += v.Data.Marshal(b[size:])

	return
}

type Timestamp struct {
	time.Time
}

func (v *Timestamp) Unmarshal(b []byte) (int, error) {
	v.Time = time.Unix(
		int64(binary.BigEndian.Uint32(b)),
		int64(binary.BigEndian.Uint32(b[4:])),
	)

	return 8, nil
}

func (v *Timestamp) Marshal(b []byte) (size int) {
	binary.BigEndian.PutUint32(b, uint32(v.Time.Unix()))
	binary.BigEndian.PutUint32(b[4:], uint32(v.Time.Nanosecond()))

	return 8
}

type Ack struct {
	Address     Address
	DelaySample Timestamp
}

func (v *Ack) Type() MessageType {
	return ACK
}

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

func (v *Ack) Marshal(b []byte) (size int) {
	size += v.Address.Marshal(b)
	size += v.DelaySample.Marshal(b[size:])

	return
}

type Have struct {
	Address
}

func (v *Have) Type() MessageType {
	return HAVE
}

type Request struct {
	Address
}

func (v *Request) Type() MessageType {
	return REQUEST
}

type Cancel struct {
	Address
}

func (v *Cancel) Type() MessageType {
	return CANCEL
}

type Empty struct{}

func (v *Empty) Unmarshal(b []byte) (int, error) {
	return 0, nil
}

func (v *Empty) Marshal(b []byte) int {
	return 0
}

type Choke struct {
	Empty
}

func (v *Choke) Type() MessageType {
	return CHOKE
}

type Unchoke struct {
	Empty
}

func (v *Unchoke) Type() MessageType {
	return UNCHOKE
}

type Messages []Message

func (v *Messages) Unmarshal(b []byte) (size int, err error) {
	for {
		if len(b) == 0 {
			return
		}

		var msg Message
		switch MessageType(b[0]) {
		case HANDSHAKE:
			msg = &Handshake{}
		case DATA:
			msg = &Data{
				Address: newAddress(),
			}
		case ACK:
			msg = &Ack{
				Address: newAddress(),
			}
		case HAVE:
			msg = &Have{
				Address: newAddress(),
			}
		case REQUEST:
			msg = &Request{
				Address: newAddress(),
			}
		case CANCEL:
			msg = &Cancel{}
		case CHOKE:
			msg = &Choke{}
		case UNCHOKE:
			msg = &Unchoke{}
		default:
			return size, ErrUnsupportedMessageType
		}

		n, err := msg.Unmarshal(b[1:])
		if err != nil {
			return size, err
		}

		*v = append(*v, msg)
		b = b[n+1:]
		size += n
	}
}

func (v *Messages) Marshal(b []byte) (size int) {
	for _, m := range *v {
		b[0] = uint8(m.Type())
		size += 1
		b = b[1:]

		n := m.Marshal(b)

		size += n
		b = b[n:]
	}

	return
}

type Datagram struct {
	ChannelID uint32
	Messages  Messages
}

func (v *Datagram) Unmarshal(b []byte) (size int, err error) {
	v.ChannelID = binary.BigEndian.Uint32(b)
	size += 4

	n, err := v.Messages.Unmarshal(b[size:])
	size += n

	return
}

func (v *Datagram) Marshal(b []byte) (size int) {
	binary.BigEndian.PutUint32(b, v.ChannelID)
	size += 4

	n := v.Messages.Marshal(b[size:])
	size += n

	return
}

func newAddress() Address {
	var a Bin32ChunkAddress
	return &a
}
