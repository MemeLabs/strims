// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package codec

// MessageHandler ...
type MessageHandler interface {
	HandleHandshake(v Handshake) error
	HandleRestart(v Restart) error
	HandleData(v Data) error
	HandleAck(v Ack) error
	HandleHave(v Have) error
	HandleIntegrity(v Integrity) error
	HandleSignedIntegrity(v SignedIntegrity) error
	HandleRequest(v Request) error
	HandleCancel(v Cancel) error
	HandleChoke(v Choke) error
	HandleUnchoke(v Unchoke) error
	HandlePing(v Ping) error
	HandlePong(v Pong) error
	HandleStreamRequest(v StreamRequest) error
	HandleStreamCancel(v StreamCancel) error
	HandleStreamOpen(v StreamOpen) error
	HandleStreamClose(v StreamClose) error
}

// Reader ...
type Reader struct {
	ChunkSize              int
	IntegrityHashSize      int
	IntegritySignatureSize int
	Handler                MessageHandler
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
		case RestartMessage:
			mn, err = v.readRestart(b[n:])
		case DataMessage:
			mn, err = v.readData(b[n:])
		case AckMessage:
			mn, err = v.readAck(b[n:])
		case HaveMessage:
			mn, err = v.readHave(b[n:])
		case IntegrityMessage:
			mn, err = v.readIntegrity(b[n:])
		case SignedIntegrityMessage:
			mn, err = v.readSignedIntegrity(b[n:])
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
		case StreamRequestMessage:
			mn, err = v.readStreamRequest(b[n:])
		case StreamCancelMessage:
			mn, err = v.readStreamCancel(b[n:])
		case StreamOpenMessage:
			mn, err = v.readStreamOpen(b[n:])
		case StreamCloseMessage:
			mn, err = v.readStreamClose(b[n:])
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

func (v Reader) readRestart(b []byte) (int, error) {
	var msg Restart
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	err = v.Handler.HandleRestart(msg)
	return n, err
}

func (v Reader) readData(b []byte) (int, error) {
	msg := Data{chunkSize: v.ChunkSize}
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	err = v.Handler.HandleData(msg)
	return n, err
}

func (v Reader) readAck(b []byte) (int, error) {
	var msg Ack
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	err = v.Handler.HandleAck(msg)
	return n, err
}

func (v Reader) readHave(b []byte) (int, error) {
	var msg Have
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	err = v.Handler.HandleHave(msg)
	return n, err
}

func (v Reader) readIntegrity(b []byte) (int, error) {
	msg := Integrity{hashSize: v.IntegrityHashSize}
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	err = v.Handler.HandleIntegrity(msg)
	return n, err
}

func (v Reader) readSignedIntegrity(b []byte) (int, error) {
	msg := SignedIntegrity{signatureSize: v.IntegritySignatureSize}
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	err = v.Handler.HandleSignedIntegrity(msg)
	return n, err
}

func (v Reader) readRequest(b []byte) (int, error) {
	var msg Request
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	err = v.Handler.HandleRequest(msg)
	return n, err
}

func (v Reader) readCancel(b []byte) (int, error) {
	var msg Cancel
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	err = v.Handler.HandleCancel(msg)
	return n, err
}

func (v Reader) readChoke(b []byte) (int, error) {
	var msg Choke
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	err = v.Handler.HandleChoke(msg)
	return n, err
}

func (v Reader) readUnchoke(b []byte) (int, error) {
	var msg Unchoke
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	err = v.Handler.HandleUnchoke(msg)
	return n, err
}

func (v Reader) readPing(b []byte) (int, error) {
	var msg Ping
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	err = v.Handler.HandlePing(msg)
	return n, err
}

func (v Reader) readPong(b []byte) (int, error) {
	var msg Pong
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	err = v.Handler.HandlePong(msg)
	return n, err
}

func (v Reader) readStreamRequest(b []byte) (int, error) {
	var msg StreamRequest
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	err = v.Handler.HandleStreamRequest(msg)
	return n, err
}

func (v Reader) readStreamCancel(b []byte) (int, error) {
	var msg StreamCancel
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	err = v.Handler.HandleStreamCancel(msg)
	return n, err
}

func (v Reader) readStreamOpen(b []byte) (int, error) {
	var msg StreamOpen
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	err = v.Handler.HandleStreamOpen(msg)
	return n, err
}

func (v Reader) readStreamClose(b []byte) (int, error) {
	var msg StreamClose
	n, err := msg.Unmarshal(b)
	if err != nil {
		return 0, err
	}
	err = v.Handler.HandleStreamClose(msg)
	return n, err
}
