// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package errutil

import "errors"

func RecoverError(v any) error {
	switch err := v.(type) {
	case nil:
		return nil
	case error:
		return err
	case string:
		return errors.New(err)
	default:
		return errors.New("unknown error")
	}
}
