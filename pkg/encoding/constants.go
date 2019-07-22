package encoding

type MessageType uint8

func (m MessageType) String() string {
	switch m {
	case HANDSHAKE:
		return "HANDSHAKE"
	case DATA:
		return "DATA"
	case ACK:
		return "ACK"
	case HAVE:
		return "HAVE"
	case INTEGRITY:
		return "INTEGRITY"
	case PEX_RESv4:
		return "PEX_RESv4"
	case PEX_REQ:
		return "PEX_REQ"
	case SIGNED_INTEGRITY:
		return "SIGNED_INTEGRITY"
	case REQUEST:
		return "REQUEST"
	case CANCEL:
		return "CANCEL"
	case CHOKE:
		return "CHOKE"
	case UNCHOKE:
		return "UNCHOKE"
	case PEX_RESv6:
		return "PEX_RESv6"
	case PEX_REScert:
		return "PEX_REScert"
	}
	panic("invalid message type")
}

const (
	HANDSHAKE MessageType = iota
	DATA
	ACK
	HAVE
	INTEGRITY
	PEX_RESv4
	PEX_REQ
	SIGNED_INTEGRITY
	REQUEST
	CANCEL
	CHOKE
	UNCHOKE
	PEX_RESv6
	PEX_REScert
)

const ChunkSize int = 1024

type ProtocolOptionType uint8

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
