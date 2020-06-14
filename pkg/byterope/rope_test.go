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
	check(t, New([]byte("autumnmajora")).Slice(4, 8)[0], []byte("mnma"))
}

func TestCopy(t *testing.T) {
	input := []byte("autumn")
	r := New(input)

	next := []byte("majora")
	n := r.Copy(next)
	if n != len(input) {
		t.Errorf("Copy: got: %d; want: %d", n, len(input))
	}
	check(t, r[0], next)

	next = []byte("just wanted to be god")
	n = r.Copy(next)
	if n != len(input) {
		t.Errorf("Copy: got: %d; want: %d", n, len(input))
	}
	check(t, r[0], next[:len(input)])
}

func BenchmarkSlice(b *testing.B) {
	b.ReportAllocs()
	input := "majora tuna autumn"
	for i := 0; i < b.N; i++ {
		for x := 1; x < len(input); x++ {
			New([]byte(input)).Slice(0, x)
		}
	}
}
func BenchmarkCopy(b *testing.B) {
	b.ReportAllocs()
	input := "majora tuna autumn"
	for i := 0; i < b.N; i++ {
		for x := 1; x < len(input); x++ {
			New([]byte(input)).Copy([]byte(input)[:x])
		}
	}
}
