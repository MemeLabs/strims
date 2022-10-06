// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package dao

import (
	"crypto/ed25519"
	"crypto/rand"
	"encoding/base64"
	"errors"
	"fmt"
	"strconv"
	"strings"

	replicationv1 "github.com/MemeLabs/strims/pkg/apis/replication/v1"
	"github.com/MemeLabs/strims/pkg/apis/type/key"
	"github.com/MemeLabs/strims/pkg/kv"
)

const CurrentVersion = 5
const MinCompatibleVersion = 5

// IDGenerator ...
type IDGenerator interface {
	GenerateID() (uint64, error)
}

type IDGeneratorFunc func() (uint64, error)

func (f IDGeneratorFunc) GenerateID() (uint64, error) { return f() }

// GenerateKey ...
func GenerateKey() (*key.Key, error) {
	pub, priv, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		return nil, err
	}

	k := &key.Key{
		Type:    key.KeyType_KEY_TYPE_ED25519,
		Private: priv,
		Public:  pub,
	}
	return k, nil
}

// Errors multi error utility
type Errors []error

func (e Errors) Error() string {
	var b strings.Builder
	var delim string
	duplicates := map[error]struct{}{}

	for i := range e {
		if _, ok := duplicates[e[i]]; ok {
			continue
		}
		duplicates[e[i]] = struct{}{}

		b.WriteString(delim)
		delim = ", "
		b.WriteString(e[i].Error())
	}

	return b.String()
}

// Includes ...
func (e Errors) Includes(err error) bool {
	for i := range e {
		if errors.Is(e[i], err) {
			return true
		}
	}
	return false
}

// IncludesOnly ...
func (e Errors) IncludesOnly(err error) bool {
	for i := range e {
		if !errors.Is(e[i], err) {
			return false
		}
	}
	return true
}

type namespace int64

func (n namespace) String() string {
	return strconv.FormatInt(int64(n), 36)
}

func (n namespace) Format(ks ...any) string {
	var b strings.Builder
	b.WriteString(n.String())
	for _, k := range ks {
		b.WriteString(":")
		switch k := k.(type) {
		case uint64:
			b.WriteString(strconv.FormatUint(k, 36))
		case string:
			b.WriteString(k)
		case []byte:
			b.WriteString(base64.RawStdEncoding.EncodeToString(k))
		case fmt.Stringer:
			b.WriteString(k.String())
		default:
			panic(fmt.Sprintf("unsupported key type %T", k))
		}
	}
	return b.String()
}

func (n namespace) FormatPrefix(ks ...any) string {
	ksc := make([]any, len(ks)+1)
	copy(ksc, ks)
	ksc[len(ks)] = ""
	return n.Format(ksc...)
}

const (
	_ namespace = iota * 1000
	systemNS
	profileNS
	certificateNS
	networkNS
	notificationNS
	chatNS
	videoNS
	vnicNS
	directoryNS
	autoseedNS
	swarmNS
	debugNS
)

const (
	_ = iota + systemNS
	mutexNS
	storeVersionNS
	replicationLogNS
	replicationVersionNS
)

type Store interface {
	kv.RWStore
	IDGenerator
	Salter
	BlobStore() kv.BlobStore
	Key() *StorageKey
	ReplicaID() uint64
	EventFilter(offset ReplicationEventFilter) ReplicationEventFilter
	ApplyEvents(es []*replicationv1.Event, c *replicationv1.Checkpoint) (*replicationv1.Checkpoint, error)
	ApplyEventLogs(ls []*replicationv1.EventLog) (*replicationv1.Checkpoint, error)
	Dump() ([]*replicationv1.Event, error)
}
