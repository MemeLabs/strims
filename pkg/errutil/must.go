// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package errutil

func Must[T any](v T, err error) T {
	if err != nil {
		panic(err)
	}
	return v
}
