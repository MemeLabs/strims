// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package bytesutil

import "bytes"

func Clamp(v, min, max []byte) []byte {
	if bytes.Compare(v, min) < 0 {
		return min
	}
	if bytes.Compare(v, max) > 0 {
		return max
	}
	return v
}
