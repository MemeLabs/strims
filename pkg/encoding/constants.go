package encoding

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
	case PExResV4Message:
		return "PExResV4"
	case PExReqMessage:
		return "PExReq"
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
	case PExResV6Message:
		return "PExResV6"
	case PExResCertMessage:
		return "PExResCert"
	case PingMessage:
		return "PingMessage"
	case PongMessage:
		return "PongMessage"
	case EndMessage:
		return "EndMessage"
	}
	panic("invalid message type")
}

// message types
const (
	HandshakeMessage MessageType = iota
	DataMessage
	AckMessage
	HaveMessage
	IntegrityMessage
	PExResV4Message
	PExReqMessage
	SignedIntegrityMessage
	RequestMessage
	CancelMessage
	ChokeMessage
	UnchokeMessage
	PExResV6Message
	PExResCertMessage
	PingMessage
	PongMessage
	EndMessage MessageType = 255
)

// ChunkSize ...
const ChunkSize int = 1024

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
	case LiveDiscardWindowOption:
		return "LiveDiscardWindow"
	case SupportedMessagesOption:
		return "SupportedMessages"
	case ChunkSizeOption:
		return "ChunkSize"
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
	LiveDiscardWindowOption
	SupportedMessagesOption
	ChunkSizeOption
	EndOption ProtocolOptionType = 255
)
