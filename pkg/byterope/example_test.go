package byterope_test

import "github.com/MemeLabs/go-ppspp/pkg/byterope"

func ExampleRope_Slice() {
	byterope.New([]byte("first chunk"), []byte("second chunk")).Slice(5, 10)
}

func ExampleRope_Copy() {
	src := byterope.New([]byte("first chunk"), []byte("second chunk"))
	dst := byterope.New(make([]byte, src.Len()/2))
	dst.Copy(src...)
}
