package dao

import (
	"crypto/ed25519"
	"crypto/rand"
	"errors"
	"strings"

	"github.com/MemeLabs/go-ppspp/pkg/apis/type/key"
)

// IDGenerator ...
type IDGenerator interface {
	GenerateID() (uint64, error)
}

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
