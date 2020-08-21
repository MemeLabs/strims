package dao

import (
	"crypto/ed25519"
	"crypto/rand"
	"sync/atomic"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

var nextSnowflakeID uint64

// generate a 53 bit locally unique id
func GenerateSnowflake() (uint64, error) {
	seconds := uint64(time.Since(time.Date(2020, 0, 0, 0, 0, 0, 0, time.UTC)) / time.Second)
	sequence := atomic.AddUint64(&nextSnowflakeID, 1) << 32
	return (seconds | sequence) & 0x1fffffffffffff, nil
}

// GenerateKey ...
func GenerateKey() (*pb.Key, error) {
	pub, priv, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		return nil, err
	}

	k := &pb.Key{
		Type:    pb.KeyType_KEY_TYPE_ED25519,
		Private: priv,
		Public:  pub,
	}
	return k, nil
}
