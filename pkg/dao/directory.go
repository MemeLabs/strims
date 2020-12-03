package dao

import (
	"crypto/ed25519"
	"encoding/binary"
	"errors"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
)

// SignDirectoryListing ...
func SignDirectoryListing(l *pb.DirectoryListing, key *pb.Key) error {
	b, _ := serializeDirectoryListing(l)

	switch key.Type {
	case pb.KeyType_KEY_TYPE_ED25519:
		if len(key.Private) != ed25519.PrivateKeySize {
			return ErrInvalidKeyLength
		}
		l.Key = key.Public
		l.Signature = ed25519.Sign(key.Private, b)
	default:
		return ErrUnsupportedKeyType
	}
	return nil
}

// VerifyDirectoryListing ...
func VerifyDirectoryListing(l *pb.DirectoryListing) error {
	b, _ := serializeDirectoryListing(l)

	if len(l.Key) != ed25519.PublicKeySize {
		return ErrInvalidKeyLength
	}
	if !ed25519.Verify(l.Key, b, l.Key) {
		return errors.New("invalid signature")
	}
	return nil
}

func serializedDirectoryListingSize(l *pb.DirectoryListing) int {
	n := len(l.Key) + len([]byte(l.MimeType)) + len([]byte(l.Title)) + len([]byte(l.Description)) + len(l.Extra) + 8
	for _, t := range l.Tags {
		n += len([]byte(t))
	}
	return n
}

// serializeDirectoryListing returns a stable byte representation of a lificate
func serializeDirectoryListing(l *pb.DirectoryListing) ([]byte, int) {
	b := make([]byte, serializedDirectoryListingSize(l))

	var n int
	n += copy(b[n:], l.Key)
	n += copy(b[n:], []byte(l.MimeType))
	n += copy(b[n:], []byte(l.Title))
	n += copy(b[n:], []byte(l.Description))
	for _, t := range l.Tags {
		n += copy(b[n:], []byte(t))
	}
	binary.BigEndian.PutUint64(b[n:], uint64(l.Timestamp))
	n += 8
	n += copy(b[n:], l.Extra)

	return b, n
}
