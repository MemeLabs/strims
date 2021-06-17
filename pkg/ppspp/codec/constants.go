package codec

import "time"

// MessageType ...
type MessageType uint8

// String ...
func (m MessageType) String() string {
	switch m {
	case HandshakeMessage:
		return "Handshake"
	case DataMessage:
		return "Data"
	case AckMessage:
		return "Ack"
	case HaveMessage:
		return "Have"
	case IntegrityMessage:
		return "Integrity"
	case SignedIntegrityMessage:
		return "SignedIntegrity"
	case RequestMessage:
		return "Request"
	case CancelMessage:
		return "Cancel"
	case ChokeMessage:
		return "Choke"
	case UnchokeMessage:
		return "Unchoke"
	case PingMessage:
		return "Ping"
	case PongMessage:
		return "Pong"
	case StreamRequestMessage:
		return "StreamRequest"
	case StreamCancelMessage:
		return "StreamCancel"
	case StreamOpenMessage:
		return "StreamOpen"
	case StreamCloseMessage:
		return "StreamClose"
	case EndMessage:
		return "End"
	}
	panic("invalid message type")
}

// message types
const (
	_ MessageType = iota
	HandshakeMessage
	DataMessage
	AckMessage
	HaveMessage
	IntegrityMessage
	SignedIntegrityMessage
	RequestMessage
	CancelMessage
	ChokeMessage
	UnchokeMessage
	PingMessage
	PongMessage
	StreamRequestMessage
	StreamCancelMessage
	StreamOpenMessage
	StreamCloseMessage
	EndMessage MessageType = 255
)

// ProtocolOptionType ...
type ProtocolOptionType uint8

// String ...
func (m ProtocolOptionType) String() string {
	switch m {
	case VersionOption:
		return "Version"
	case MinimumVersionOption:
		return "MinimumVersion"
	case SwarmIdentifierOption:
		return "SwarmIdentifier"
	case ContentIntegrityProtectionMethodOption:
		return "ContentIntegrityProtectionMethod"
	case MerkleHashTreeFunctionOption:
		return "MerkleHashTreeFunction"
	case LiveSignatureAlgorithmOption:
		return "LiveSignatureAlgorithm"
	case ChunkAddressingMethodOption:
		return "ChunkAddressingMethod"
	case LiveWindowOption:
		return "LiveWindow"
	case SupportedMessagesOption:
		return "SupportedMessages"
	case ChunkSizeOption:
		return "ChunkSize"
	case ChunksPerSignatureOption:
		return "ChunksPerSignature"
	case StreamCountOption:
		return "StreamCount"
	case EndOption:
		return "EndOption"
	}
	panic("invalid protocol option")
}

// protocol options
const (
	VersionOption ProtocolOptionType = iota
	MinimumVersionOption
	SwarmIdentifierOption
	ContentIntegrityProtectionMethodOption
	MerkleHashTreeFunctionOption
	LiveSignatureAlgorithmOption
	ChunkAddressingMethodOption
	LiveWindowOption
	SupportedMessagesOption
	ChunkSizeOption
	ChunksPerSignatureOption
	StreamCountOption
	EndOption ProtocolOptionType = 255
)

const timeGranularity = int64(time.Millisecond)
