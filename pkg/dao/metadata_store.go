package dao

import (
	"errors"
)

const metadataTable = "default"

// errors ...
var (
	ErrProfileNameNotAvailable = errors.New("profile name not available")
	ErrAuthenticationRequired  = errors.New("method requires authentication")
)

// NewMetadataStore ...
func NewMetadataStore(store BlobStore) (*MetadataStore, error) {
	if err := store.CreateStoreIfNotExists(metadataTable); err != nil {
		return nil, err
	}

	return &MetadataStore{
		BlobStore: store,
	}, nil
}

// MetadataStore ...
type MetadataStore struct {
	BlobStore
}
