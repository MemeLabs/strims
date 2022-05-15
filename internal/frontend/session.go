// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package frontend

import (
	"encoding/base64"
	"errors"
	"strconv"
	"strings"

	"github.com/MemeLabs/strims/internal/dao"
	profilev1 "github.com/MemeLabs/strims/pkg/apis/profile/v1"
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
	storageKey, _ := dao.NewStorageKeyFromBytes(kb, nil)

	return profileID, storageKey, nil
}

func marshalSessionID(profile *profilev1.Profile, store *dao.ProfileStore) string {
	id := strconv.FormatUint(profile.Id, 36)
	storageKey := base64.RawURLEncoding.EncodeToString(store.Key().Key())
	return id + "." + storageKey
}
