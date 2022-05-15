// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package ppspptest

import (
	"encoding/json"
	"io"
	"log"

	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"go.uber.org/zap"
)

// Key ...
func Key() *key.Key {
	key := &key.Key{}
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
	HandleMessage(b []byte) error
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
			log.Println(err)
			return
		}
		if err := ch.HandleMessage(b[:n]); err != nil {
			log.Println("handling message failed", err)
		}
	}
}
