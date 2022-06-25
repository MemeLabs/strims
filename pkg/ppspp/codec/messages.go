// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package codec

import (
	"encoding/binary"
	"errors"
	"time"

	"github.com/MemeLabs/strims/pkg/binaryutil"
	"github.com/MemeLabs/strims/pkg/binmap"
	"github.com/MemeLabs/strims/pkg/timeutil"
)

// errors ...
var (
	ErrUnsupportedMessageType    = errors.New("unsupported message type")
	ErrUnsupportedProtocolOption = errors.New("unsupported protocol option")
)

// Decoder ...
type Decoder interface {
	Unmarshal(b []byte) (int, error)
}

// Encoder ...
type Encoder interface {
	Marshal(b []byte) int
}

// Message ...
type Message interface {
	Decoder
	Encoder
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
	vi, n := binary.Uvarint(b)
	*v = Address(vi)
	return n, nil
}

// Marshal ...
func (v Address) Marshal(b []byte) int {
	return binary.PutUvarint(b, uint64(v))
}

// ByteLen ...
func (v Address) ByteLen() int {
	return binaryutil.UvarintLen(uint64(v))
}

// Bin ...
func (v Address) Bin() binmap.Bin {
	return binmap.Bin(v)
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
	Decoder
	Encoder
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

// MustFind ...
func (o ProtocolOptions) MustFind(t ProtocolOptionType) ProtocolOption {
	opt, ok := o.Find(t)
	if !ok {
		panic("protocol option does not found")
	}
	return opt
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

// LiveWindowProtocolOption ...
type LiveWindowProtocolOption struct {
	Value uint32
}

// Unmarshal ...
func (v *LiveWindowProtocolOption) Unmarshal(b []byte) (int, error) {
	v.Value = binary.BigEndian.Uint32(b)
	return 4, nil
}

// Marshal ...
func (v *LiveWindowProtocolOption) Marshal(b []byte) int {
	binary.BigEndian.PutUint32(b, v.Value)
	return 4
}

// Type ...
func (v *LiveWindowProtocolOption) Type() ProtocolOptionType {
	return LiveWindowOption
}

// ByteLen ...
func (v *LiveWindowProtocolOption) ByteLen() int {
	return 4
}

// ChunkSizeProtocolOption ...
type ChunkSizeProtocolOption struct {
	Value uint32
}

// Unmarshal ...
func (v *ChunkSizeProtocolOption) Unmarshal(b []byte) (int, error) {
	v.Value = binary.BigEndian.Uint32(b)
	return 4, nil
}

// Marshal ...
func (v *ChunkSizeProtocolOption) Marshal(b []byte) int {
	binary.BigEndian.PutUint32(b, v.Value)
	return 4
}

// Type ...
func (v *ChunkSizeProtocolOption) Type() ProtocolOptionType {
	return ChunkSizeOption
}

// ByteLen ...
func (v *ChunkSizeProtocolOption) ByteLen() int {
	return 4
}

// ChunksPerSignatureProtocolOption ...
type ChunksPerSignatureProtocolOption struct {
	Value uint32
}

// Unmarshal ...
func (v *ChunksPerSignatureProtocolOption) Unmarshal(b []byte) (int, error) {
	v.Value = binary.BigEndian.Uint32(b)
	return 4, nil
}

// Marshal ...
func (v *ChunksPerSignatureProtocolOption) Marshal(b []byte) int {
	binary.BigEndian.PutUint32(b, v.Value)
	return 4
}

// Type ...
func (v *ChunksPerSignatureProtocolOption) Type() ProtocolOptionType {
	return ChunksPerSignatureOption
}

// ByteLen ...
func (v *ChunksPerSignatureProtocolOption) ByteLen() int {
	return 4
}

// StreamCountProtocolOption ...
type StreamCountProtocolOption struct {
	Value uint16
}

// Unmarshal ...
func (v *StreamCountProtocolOption) Unmarshal(b []byte) (int, error) {
	v.Value = binary.BigEndian.Uint16(b)
	return 2, nil
}

// Marshal ...
func (v *StreamCountProtocolOption) Marshal(b []byte) int {
	binary.BigEndian.PutUint16(b, v.Value)
	return 2
}

// Type ...
func (v *StreamCountProtocolOption) Type() ProtocolOptionType {
	return StreamCountOption
}

// ByteLen ...
func (v *StreamCountProtocolOption) ByteLen() int {
	return 2
}

// ContentIntegrityProtectionMethodProtocolOption ...
type ContentIntegrityProtectionMethodProtocolOption struct {
	Value uint8
}

// Unmarshal ...
func (v *ContentIntegrityProtectionMethodProtocolOption) Unmarshal(b []byte) (int, error) {
	v.Value = b[0]
	return 1, nil
}

// Marshal ...
func (v *ContentIntegrityProtectionMethodProtocolOption) Marshal(b []byte) int {
	b[0] = v.Value
	return 1
}

// Type ...
func (v *ContentIntegrityProtectionMethodProtocolOption) Type() ProtocolOptionType {
	return ContentIntegrityProtectionMethodOption
}

// ByteLen ...
func (v *ContentIntegrityProtectionMethodProtocolOption) ByteLen() int {
	return 1
}

// MerkleHashTreeFunctionProtocolOption ...
type MerkleHashTreeFunctionProtocolOption struct {
	Value uint8
}

// Unmarshal ...
func (v *MerkleHashTreeFunctionProtocolOption) Unmarshal(b []byte) (int, error) {
	v.Value = b[0]
	return 1, nil
}

// Marshal ...
func (v *MerkleHashTreeFunctionProtocolOption) Marshal(b []byte) int {
	b[0] = v.Value
	return 1
}

// Type ...
func (v *MerkleHashTreeFunctionProtocolOption) Type() ProtocolOptionType {
	return MerkleHashTreeFunctionOption
}

// ByteLen ...
func (v *MerkleHashTreeFunctionProtocolOption) ByteLen() int {
	return 1
}

// LiveSignatureAlgorithmProtocolOption ...
type LiveSignatureAlgorithmProtocolOption struct {
	Value uint8
}

// Unmarshal ...
func (v *LiveSignatureAlgorithmProtocolOption) Unmarshal(b []byte) (int, error) {
	v.Value = b[0]
	return 1, nil
}

// Marshal ...
func (v *LiveSignatureAlgorithmProtocolOption) Marshal(b []byte) int {
	b[0] = v.Value
	return 1
}

// Type ...
func (v *LiveSignatureAlgorithmProtocolOption) Type() ProtocolOptionType {
	return LiveSignatureAlgorithmOption
}

// ByteLen ...
func (v *LiveSignatureAlgorithmProtocolOption) ByteLen() int {
	return 1
}

// NewEpochProtocolOption ...
func NewEpochProtocolOption(t timeutil.Time, sig []byte) *EpochProtocolOption {
	if t.IsNil() {
		return nil
	}
	return &EpochProtocolOption{
		Timestamp: Timestamp{t},
		Signature: sig,
	}
}

// EpochProtocolOption ...
type EpochProtocolOption struct {
	signatureSize int
	Timestamp     Timestamp
	Signature     Buffer
}

// Unmarshal ...
func (v *EpochProtocolOption) Unmarshal(b []byte) (size int, err error) {
	n, err := v.Timestamp.Unmarshal(b)
	if err != nil {
		return
	}
	size += n

	v.Signature = b[size : size+v.signatureSize]
	size += v.signatureSize

	return
}

// Marshal ...
func (v *EpochProtocolOption) Marshal(b []byte) (size int) {
	size += v.Timestamp.Marshal(b)
	size += v.Signature.Marshal(b[size:])
	return
}

// Type ...
func (v *EpochProtocolOption) Type() ProtocolOptionType {
	return EpochOption
}

// ByteLen ...
func (v *EpochProtocolOption) ByteLen() int {
	return v.Timestamp.ByteLen() + v.Signature.ByteLen()
}

// NewSwarmIdentifierProtocolOption ...
func NewSwarmIdentifierProtocolOption(id []byte) *SwarmIdentifierProtocolOption {
	o := SwarmIdentifierProtocolOption(id)
	return &o
}

// SwarmIdentifierProtocolOption ...
type SwarmIdentifierProtocolOption []byte

// Unmarshal ...
func (v *SwarmIdentifierProtocolOption) Unmarshal(b []byte) (size int, err error) {
	idSize := int(binary.BigEndian.Uint16(b))
	size += 2

	*v = b[size : size+idSize]
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
	signatureSize int
	ChannelID     uint32
	Options       ProtocolOptions
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

	for size < len(b) {
		optionType := ProtocolOptionType(b[size])
		size++

		var option ProtocolOption
		switch optionType {
		case VersionOption:
			option = &VersionProtocolOption{}
		case MinimumVersionOption:
			option = &MinimumVersionProtocolOption{}
		case SwarmIdentifierOption:
			option = &SwarmIdentifierProtocolOption{}
		case LiveWindowOption:
			option = &LiveWindowProtocolOption{}
		case ChunkSizeOption:
			option = &ChunkSizeProtocolOption{}
		case ChunksPerSignatureOption:
			option = &ChunksPerSignatureProtocolOption{}
		case StreamCountOption:
			option = &StreamCountProtocolOption{}
		case ContentIntegrityProtectionMethodOption:
			option = &ContentIntegrityProtectionMethodProtocolOption{}
		case MerkleHashTreeFunctionOption:
			option = &MerkleHashTreeFunctionProtocolOption{}
		case LiveSignatureAlgorithmOption:
			option = &LiveSignatureAlgorithmProtocolOption{}
		case EpochOption:
			option = &EpochProtocolOption{signatureSize: v.signatureSize}
		case EndOption:
			return
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

	b[size] = byte(EndOption)
	size++

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
	chunkSize int
	Address   Address
	Timestamp Timestamp
	Data      Buffer
}

// NewData ...
func NewData(chunkSize int, b binmap.Bin, t timeutil.Time, d []byte) *Data {
	return &Data{
		chunkSize: chunkSize,
		Address:   Address(b),
		Timestamp: Timestamp{t},
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

	n = int(v.Address.Bin().BaseLength()) * v.chunkSize
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
	timeutil.Time
}

// Unmarshal ...
func (v *Timestamp) Unmarshal(b []byte) (int, error) {
	vi, n := binary.Varint(b)
	v.Time = timeutil.New(vi * int64(timeutil.Precision))
	return n, nil
}

// Marshal ...
func (v Timestamp) Marshal(b []byte) int {
	return binary.PutVarint(b, v.Time.UnixNano()/int64(timeutil.Precision))
}

// ByteLen ...
func (v Timestamp) ByteLen() int {
	return binaryutil.VarintLen(v.Time.UnixNano() / int64(timeutil.Precision))
}

// DelaySample ...
type DelaySample struct {
	time.Duration
}

// Unmarshal ...
func (v *DelaySample) Unmarshal(b []byte) (int, error) {
	vi, n := binary.Varint(b)
	v.Duration = time.Duration(vi * int64(timeutil.Precision))
	return n, nil
}

// Marshal ...
func (v DelaySample) Marshal(b []byte) int {
	return binary.PutVarint(b, int64(v.Duration)/int64(timeutil.Precision))
}

// ByteLen ...
func (v DelaySample) ByteLen() int {
	return binaryutil.VarintLen(int64(v.Duration) / int64(timeutil.Precision))
}

// Ack ...
type Ack struct {
	Address     Address
	DelaySample DelaySample
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

	n, err = v.DelaySample.Unmarshal(b[size:])
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

// Integrity ...
type Integrity struct {
	hashSize int
	Address  Address
	Hash     Buffer
}

// Type ...
func (v *Integrity) Type() MessageType {
	return IntegrityMessage
}

// Unmarshal ...
func (v *Integrity) Unmarshal(b []byte) (size int, err error) {
	n, err := v.Address.Unmarshal(b)
	if err != nil {
		return
	}
	size += n

	v.Hash = b[size : size+v.hashSize]
	size += v.hashSize

	return
}

// Marshal ...
func (v *Integrity) Marshal(b []byte) (size int) {
	size += v.Address.Marshal(b)
	size += v.Hash.Marshal(b[size:])

	return
}

// ByteLen ...
func (v *Integrity) ByteLen() int {
	return v.Address.ByteLen() + v.Hash.ByteLen()
}

// SignedIntegrity ...
type SignedIntegrity struct {
	signatureSize int
	Address       Address
	Timestamp     Timestamp
	Signature     Buffer
}

// Type ...
func (v *SignedIntegrity) Type() MessageType {
	return SignedIntegrityMessage
}

// Unmarshal ...
func (v *SignedIntegrity) Unmarshal(b []byte) (size int, err error) {
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

	v.Signature = b[size : size+v.signatureSize]
	size += v.signatureSize

	return
}

// Marshal ...
func (v *SignedIntegrity) Marshal(b []byte) (size int) {
	size += v.Address.Marshal(b)
	size += v.Timestamp.Marshal(b[size:])
	size += v.Signature.Marshal(b[size:])

	return
}

// ByteLen ...
func (v *SignedIntegrity) ByteLen() int {
	return v.Address.ByteLen() + v.Timestamp.ByteLen() + v.Signature.ByteLen()
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
	Nonce Nonce
	Delay uint64
}

// Type ...
func (v *Pong) Type() MessageType {
	return PongMessage
}

// Unmarshal ...
func (v *Pong) Unmarshal(b []byte) (size int, err error) {
	n, err := v.Nonce.Unmarshal(b)
	if err != nil {
		return
	}
	size += n

	v.Delay = binary.BigEndian.Uint64(b[size:])
	size += 8

	return
}

// Marshal ...
func (v *Pong) Marshal(b []byte) (size int) {
	size += v.Nonce.Marshal(b)
	binary.BigEndian.PutUint64(b[size:], v.Delay)
	size += 8

	return
}

// ByteLen ...
func (v *Pong) ByteLen() int {
	return 16
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
	Address   Address
	Timestamp Timestamp
}

// Unmarshal ...
func (v *Request) Unmarshal(b []byte) (size int, err error) {
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

	return
}

// Marshal ...
func (v *Request) Marshal(b []byte) (size int) {
	size += v.Address.Marshal(b)
	size += v.Timestamp.Marshal(b[size:])

	return
}

// ByteLen ...
func (v *Request) ByteLen() int {
	return v.Address.ByteLen() + v.Timestamp.ByteLen()
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

// Stream ...
type Stream uint16

// NewStream ...
func NewStream(b uint16) *Stream {
	a := Stream(b)
	return &a
}

// Unmarshal ...
func (v *Stream) Unmarshal(b []byte) (int, error) {
	*v = Stream(binary.BigEndian.Uint16(b))
	return 2, nil
}

// Marshal ...
func (v Stream) Marshal(b []byte) int {
	binary.BigEndian.PutUint16(b, uint16(v))
	return 2
}

// ByteLen ...
func (v Stream) ByteLen() int {
	return 2
}

// StreamAddress ...
type StreamAddress struct {
	Stream  Stream
	Address Address
}

// Unmarshal ...
func (v *StreamAddress) Unmarshal(b []byte) (size int, err error) {
	n, err := v.Stream.Unmarshal(b)
	if err != nil {
		return
	}
	size += n

	n, err = v.Address.Unmarshal(b[size:])
	if err != nil {
		return
	}
	size += n

	return
}

// Marshal ...
func (v *StreamAddress) Marshal(b []byte) (size int) {
	size += v.Stream.Marshal(b)
	size += v.Address.Marshal(b[size:])

	return
}

// ByteLen ...
func (v *StreamAddress) ByteLen() int {
	return v.Stream.ByteLen() + v.Address.ByteLen()
}

// StreamRequest ...
type StreamRequest struct {
	StreamAddress
}

// Type ...
func (v *StreamRequest) Type() MessageType {
	return StreamRequestMessage
}

// StreamCancel ...
type StreamCancel struct {
	Stream
}

// Type ...
func (v *StreamCancel) Type() MessageType {
	return StreamCancelMessage
}

// StreamOpen ...
type StreamOpen struct {
	StreamAddress
}

// Type ...
func (v *StreamOpen) Type() MessageType {
	return StreamOpenMessage
}

// StreamClose ...
type StreamClose struct {
	Stream
}

// Type ...
func (v *StreamClose) Type() MessageType {
	return StreamCloseMessage
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

// Restart ...
type Restart struct {
	Empty
}

// Type ...
func (v *Restart) Type() MessageType {
	return RestartMessage
}

// End ...
type End struct {
	Empty
}

// Type ...
func (v *End) Type() MessageType {
	return EndMessage
}

// Channel ...
type Channel uint64

// NewChannel ...
func NewChannel(b uint64) *Channel {
	v := Channel(b)
	return &v
}

// Unmarshal ...
func (v *Channel) Unmarshal(b []byte) (int, error) {
	vi, n := binary.Uvarint(b)
	*v = Channel(vi)
	return n, nil
}

// Marshal ...
func (v Channel) Marshal(b []byte) int {
	return binary.PutUvarint(b, uint64(v))
}

// ByteLen ...
func (v Channel) ByteLen() int {
	return binaryutil.UvarintLen(uint64(v))
}

// ChannelHeader ...
type ChannelHeader struct {
	Channel Channel
	Length  uint16
}

// Unmarshal ...
func (v *ChannelHeader) Unmarshal(b []byte) (size int, err error) {
	n, err := v.Channel.Unmarshal(b)
	if err != nil {
		return
	}
	size += n

	v.Length = binary.BigEndian.Uint16(b[size:])
	size += 2

	return
}

// Marshal ...
func (v *ChannelHeader) Marshal(b []byte) (size int) {
	size += v.Channel.Marshal(b)
	binary.BigEndian.PutUint16(b[size:], v.Length)
	size += 2

	return
}

// ByteLen ...
func (v *ChannelHeader) ByteLen() int {
	return v.Channel.ByteLen() + 2
}
