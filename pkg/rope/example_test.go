// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package rope_test

import "github.com/MemeLabs/strims/pkg/rope"

func ExampleRope_Slice() {
	rope.New([]byte("first chunk"), []byte("second chunk")).Slice(5, 10)
}

func ExampleRope_Copy() {
	src := rope.New([]byte("first chunk"), []byte("second chunk"))
	dst := rope.New(make([]byte, src.Len()/2))
	dst.Copy(src...)
}
