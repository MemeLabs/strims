package codec

// MessageHandler ...
type MessageHandler interface {
	HandleHandshake(v Handshake) error
	HandleData(v Data)
	HandleAck(v Ack)
	HandleHave(v Have)
	HandleRequest(v Request)
	HandleCancel(v Cancel)
	HandleChoke(v Choke)
	HandleUnchoke(v Unchoke)
	HandlePing(v Ping)
	HandlePong(v Pong)
}

// Reader ...
type Reader struct {
	ChunkSize int
	Handler   MessageHandler
}

func (v Reader) Read(b []byte) (n int, err error) {
	for {
		if n >= len(b) {
			return
		}

		mt := MessageType(b[n])
		n++

		var mn int
		switch mt {
		case HandshakeMessage:
			mn, err = v.readHandshake(b[n:])
		case DataMessage:
			mn, err = v.readData(b[n:])
		case AckMessage:
			mn, err = v.readAck(b[n:])
		case HaveMessage:
			mn, err = v.readHave(b[n:])
		case RequestMessage:
			mn, err = v.readRequest(b[n:])
		case CancelMessage:
			mn, err = v.readCancel(b[n:])
		case ChokeMessage:
			mn, err = v.readChoke(b[n:])
		case UnchokeMessage:
			mn, err = v.readUnchoke(b[n:])
		case PingMessage:
			mn, err = v.readPing(b[n:])
		case PongMessage:
			mn, err = v.readPong(b[n:])
		case EndMessage:
			return
		default:
			return n, ErrUnsupportedMessageType
		}

		if err != nil {
			return n, err
		}
		n += mn
	}
}

func (v Reader) readHandshake(b []byte) (int, error) {
	var msg Handshake
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	err = v.Handler.HandleHandshake(msg)
	return n, err
}

func (v Reader) readData(b []byte) (int, error) {
	msg := Data{chunkSize: v.ChunkSize}
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	v.Handler.HandleData(msg)
	return n, err
}

func (v Reader) readAck(b []byte) (int, error) {
	var msg Ack
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	v.Handler.HandleAck(msg)
	return n, err
}

func (v Reader) readHave(b []byte) (int, error) {
	var msg Have
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	v.Handler.HandleHave(msg)
	return n, err
}

func (v Reader) readRequest(b []byte) (int, error) {
	var msg Request
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	v.Handler.HandleRequest(msg)
	return n, err
}

func (v Reader) readCancel(b []byte) (int, error) {
	var msg Cancel
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	v.Handler.HandleCancel(msg)
	return n, err
}

func (v Reader) readChoke(b []byte) (int, error) {
	var msg Choke
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	v.Handler.HandleChoke(msg)
	return n, err
}

func (v Reader) readUnchoke(b []byte) (int, error) {
	var msg Unchoke
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	v.Handler.HandleUnchoke(msg)
	return n, err
}

func (v Reader) readPing(b []byte) (int, error) {
	var msg Ping
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	v.Handler.HandlePing(msg)
	return n, err
}

func (v Reader) readPong(b []byte) (int, error) {
	var msg Pong
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	v.Handler.HandlePong(msg)
	return n, err
}
