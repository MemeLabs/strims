// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package rope

import (
	"bytes"
	"testing"
)

func check(t *testing.T, r []byte, want []byte) {
	t.Helper()

	if !bytes.Equal(r, want) {
		t.Errorf("got: %q; want: %q", r, want)
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

	firstHalf := make([]byte, r.Len()/2)
	New(firstHalf).Copy(r...)
	check(t, firstHalf, []byte("first chunk of filesecond chu"))
}

func TestCopiesOfVaryingSize(t *testing.T) {
	data := make([]byte, 100)
	for i := 0; i < len(data); i++ {
		data[i] = byte(i)
	}

	first := make([]byte, 20)
	second := make([]byte, 30)
	third := make([]byte, 50)

	firstRope := New(first, second, third)
	if n := firstRope.Copy(data); n != firstRope.Len() {
		t.Errorf("got %d; want: %d", n, firstRope.Len())
	}

	fourth := make([]byte, 40)
	fifth := make([]byte, 60)

	secRope := New(fourth, fifth)
	if n := secRope.Copy(firstRope...); n != firstRope.Len() {
		t.Errorf("got %d; want: %d", n, firstRope.Len())
	}

	sixth := make([]byte, 25)
	seventh := make([]byte, 25)
	eighth := make([]byte, 25)
	ninth := make([]byte, 25)

	thirdRope := New(sixth, seventh, eighth, ninth)
	if n := thirdRope.Copy(secRope...); n != secRope.Len() {
		t.Errorf("got %d; want: %d", n, secRope.Len())
	}

	tenth := make([]byte, 100)
	fourthRope := New(tenth)
	if n := fourthRope.Copy(thirdRope...); n != thirdRope.Len() {
		t.Errorf("got %d; want: %d", n, thirdRope.Len())
	}

	check(t, fourthRope[0], data)
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
