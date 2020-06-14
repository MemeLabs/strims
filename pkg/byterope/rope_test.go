package byterope

import (
	"bytes"
	"testing"
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
	r := New(x, y, z)

	fullLen := r.Len()
	firstHalf := make([]byte, fullLen/2)
	n := New(firstHalf).Copy(r...)
	if n != len(firstHalf) {
		t.Errorf("got %d; want: %d", n, len(firstHalf))
	}
	check(t, firstHalf, []byte("first chunk of filesecond chu"))
}

func BenchmarkSlice(b *testing.B) {
	b.ReportAllocs()
	x := []byte("first chunk of file")
	y := []byte("second chunk of file")
	z := []byte("third chunk of file")

	r := New(x, y, z)
	for i := 0; i < b.N; i++ {
		r.Slice(0, r.Len()/2)
	}
}
func BenchmarkCopy(b *testing.B) {
	b.ReportAllocs()
	x := []byte("first chunk of file")
	y := []byte("second chunk of file")
	z := []byte("third chunk of file")

	r := New(x, y, z)
	firstHalf := make([]byte, r.Len()/2)
	for i := 0; i < b.N; i++ {
		New(firstHalf).Copy(r...)
	}
}
