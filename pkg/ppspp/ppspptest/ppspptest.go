package ppspptest

import (
	"encoding/json"
	"io"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"go.uber.org/zap"
)

// Key ...
func Key() *pb.Key {
	key := &pb.Key{}
	err := json.Unmarshal([]byte(`{"type":1,"private":"xIbkrrbgy24ps/HizaIsik1X0oAO2CSq9bAFDHa5QtfS4l/CTqSzU7BlqiQa1cOeQR94FZCN0RJuqoYgirV+Mg==","public":"0uJfwk6ks1OwZaokGtXDnkEfeBWQjdESbqqGIIq1fjI="}`), &key)
	if err != nil {
		panic(err)
	}
	return key
}

// Logger ...
func Logger() *zap.Logger {
	logger, err := zap.NewDevelopment()
	if err != nil {
		panic(err)
	}
	return logger
}

// MessageHandler ...
type MessageHandler interface {
	HandleMessage(b []byte) (int, error)
}

// ReaderMTUer ...
type ReaderMTUer interface {
	io.Reader
	MTU() int
}

// ReadChannelConn ...
func ReadChannelConn(c ReaderMTUer, ch MessageHandler) {
	b := make([]byte, c.MTU())
	for {
		n, err := c.Read(b)
		if err != nil {
			panic(err)
		}
		ch.HandleMessage(b[:n])
	}
}
