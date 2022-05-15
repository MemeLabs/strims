// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package daotest

import (
	"log"

	"github.com/MemeLabs/strims/pkg/debug"
	"google.golang.org/protobuf/proto"
)

type MockEventEmitter struct{}

func (e *MockEventEmitter) Emit(v proto.Message) {
	log.Printf("event emitted %T", v)
	debug.PrintJSON(v)
}
