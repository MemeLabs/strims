// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package pointerutil

func To[T any](v T) *T {
	return &v
}
