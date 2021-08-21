package frontend

import (
	"encoding/base64"
	"errors"
	"strconv"
	"strings"

	"github.com/MemeLabs/go-ppspp/internal/dao"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
)

var errMalformedSessionID = errors.New("malformed session id")

func unmarshalSessionID(id string) (uint64, *dao.StorageKey, error) {
	i := strings.IndexRune(id, '.')
	if i == -1 {
		return 0, nil, errMalformedSessionID
	}

	profileID, err := strconv.ParseUint(id[:i], 36, 64)
	if err != nil {
		return 0, nil, err
	}

	kb, err := base64.RawURLEncoding.DecodeString(id[i+1:])
	if err != nil {
		return 0, nil, err
	}
	storageKey := dao.NewStorageKeyFromBytes(kb)

	return profileID, storageKey, nil
}

func marshalSessionID(profile *profilev1.Profile, store *dao.ProfileStore) string {
	id := strconv.FormatUint(profile.Id, 36)
	storageKey := base64.RawURLEncoding.EncodeToString(store.Key().Key())
	return id + "." + storageKey
}
