package byterope

import (
	"bytes"
	"testing"

	"github.com/davecgh/go-spew/spew"
)

func check(t *testing.T, r []byte, want []byte) {
	t.Helper()

	if !bytes.Equal(r, want) {
		t.Errorf("got: %q; want: %q", r, want)
	}

	if len(r) != len(want) {
		t.Errorf("Len: got %d; want: %d", len(r), len(want))
	}
}

func TestSlice(t *testing.T) {
	r := New([]byte("autumn"), []byte("majora")).Slice(4, 8)
	check(t, r[0], []byte("mn"))
	check(t, r[1], []byte("ma"))
}

func TestCopy(t *testing.T) {
	x := []byte("first chunk of file")
	y := []byte("second chunk of file")
	z := []byte("third chunk of file")
	fullLen := len(x) + len(y) + len(z)
	firstHalf := make([]byte, fullLen/2)
	n := New(x, y, z).Copy(firstHalf)
	spew.Dump(firstHalf)
	if n != len(firstHalf) {
		t.Errorf("got %d; want: %d", n, len(firstHalf))
	}
	check(t, firstHalf, []byte("first chunk of the file second chun"))
}

func BenchmarkSlice(b *testing.B) {
	b.ReportAllocs()
	x := []byte("first chunk of file")
	y := []byte("second chunk of file")
	z := []byte("third chunk of file")

	fullLen := len(x) + len(y) + len(z)
	for i := 0; i < b.N; i++ {
		New(x, y, z).Slice(0, fullLen/2)
	}
}
func BenchmarkCopy(b *testing.B) {
	b.ReportAllocs()
	x := []byte("first chunk of file")
	y := []byte("second chunk of file")
	z := []byte("third chunk of file")

	fullLen := len(x) + len(y) + len(z)
	firstHalf := make([]byte, fullLen/2)
	for i := 0; i < b.N; i++ {
		New(x, y, z).Copy(firstHalf)
	}
}
