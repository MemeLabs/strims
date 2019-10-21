package binmap

import (
	"log"
	"math"
	"testing"
)

// func TestMap_extendRoot(t *testing.T) {
// 	log.SetFlags(log.LstdFlags | log.Lshortfile)
// 	m := New()

// 	i := 12800

// 	m.Set(Bin(i))
// 	if m.Empty(Bin(i)) {
// 		t.Errorf("m.Empty(%d) should not be true", i)
// 		t.Fail()
// 	}

// 	m.extendRoot()
// 	if m.Empty(Bin(i)) {
// 		t.Errorf("m.Empty(%d) should not be true", i)
// 		t.Fail()
// 	}

// 	// spew.Dump(m)
// }

// func TestMap_reserveCells(t *testing.T) {
// 	log.SetFlags(log.LstdFlags | log.Lshortfile)
// 	m := New()

// 	for i := 0; i <= 1<<24-1; i += 2 {
// 		b := Bin(i)

// 		m.Set(b)

// 		if !m.Filled(b) {
// 			t.Errorf("m.Filled(%d) should be true", b)
// 			t.Fail()
// 		}
// 	}

// 	spew.Dump(m)
// }

func TestMap_FindEmptyAfter(t *testing.T) {
	log.SetFlags(log.LstdFlags | log.Lshortfile)

	m := New()
	for i := 0; i < 128; i += 2 {
		m.Set(Bin(i))
	}
	log.Println(m)
	b := m.FindEmptyAfter(126)
	if b != 128 {
		t.Errorf("expected 128 got %s", b)
		t.Fail()
	}
}

// func TestMap_reserveCells(t *testing.T) {
// 	log.SetFlags(log.LstdFlags | log.Lshortfile)
// 	m := New()

// 	// for i := 0; i < (1 << 24); i += 4 {
// 	// 	m.Set(Bin(i))
// 	// 	if m.Empty(Bin(i)) {
// 	// 		t.Errorf("m.Empty(%d) should not be true", i)
// 	// 		t.Fail()
// 	// 	}
// 	// 	if !m.Filled(Bin(i)) {
// 	// 		t.Errorf("m.Filled(%d) should be true", i)
// 	// 		t.Fail()
// 	// 	}
// 	// }

// 	// 10111011 10111011 10111011 10111011

// 	for i := Bin(0); i < 256; i += 2 {
// 		m.Set(i)
// 	}
// 	m.Reset(191)
// 	for i := Bin(128); i < 160; i += 2 {
// 		m.Set(i)
// 	}
// 	for i := Bin(192); i < 224; i += 2 {
// 		m.Set(i)
// 	}

// 	for i := 3; i < 256; i += 15 {
// 		log.Printf(
// 			"in: % 5d    a: % 5d    b: % 5d",
// 			i,
// 			m.FindEmptyAfter(Bin(i)),
// 		)
// 	}

// 	log.Println(m)
// }

func BenchmarkTestMap_b(b *testing.B) {
	for j := 0; j < b.N; j += 32 {
		for i := uint32(0); i < 32; i++ {
			_ = bitmapBin(bitmap(math.MaxUint32 << i))
			_ = bitmapBin(bitmap(1 << i))
		}
	}
}

func BenchmarkTestMap_reserveCells(b *testing.B) {
	var m = New()

	for n := 0; n < b.N; n++ {
		m.Set(Bin(n))

		if m.EmptyAt(Bin(n)) {
			panic("rip...")
		}
	}
}

// func BenchmarkTestMap_reserveCells(b *testing.B) {
// 	d := make([]byte, 4*b.N)
// 	rand.Read(d)
// 	b.ResetTimer()

// 	var m = New()

// 	for i := 4; i < len(d); i += 4 {
// 		n := Bin(uint64(binary.BigEndian.Uint32(d[i-4:]))) >> 8

// 		m.Set(n)

// 		if m.Empty(n) {
// 			panic("rip...")
// 		}
// 	}
// }
