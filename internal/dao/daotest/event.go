package daotest

import (
	"log"

	"github.com/MemeLabs/go-ppspp/pkg/debug"
	"google.golang.org/protobuf/proto"
)

type MockEventEmitter struct{}

func (e *MockEventEmitter) Emit(v proto.Message) {
	log.Printf("event emitted %T", v)
	debug.PrintJSON(v)
}
