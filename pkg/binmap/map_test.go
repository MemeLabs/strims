package binmap

import (
	"math/rand"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestSetGet(t *testing.T) {
	assert := assert.New(t)
	bs := New()
	b3 := NewBin(1, 0)
	b2 := NewBin(0, 1)
	bs.Set(b3)

	assert.True(bs.FilledAt(b3))
	assert.True(bs.FilledAt(b2))
}

func TestChess(t *testing.T) {
	assert := assert.New(t)
	chess16 := New()

	for i := 0; i < 16; i++ {
		if i&1 == 1 {
			chess16.Set(NewBin(0, uint64(i)))
		} else {
			chess16.Reset(NewBin(0, uint64(i)))
		}
	}

	for i := 0; i < 16; i++ {
		if i&1 == 1 {
			assert.True(chess16.FilledAt(NewBin(0, uint64(i))))
		} else {
			assert.True(chess16.EmptyAt(NewBin(0, uint64(i))))
		}
	}

	assert.False(chess16.EmptyAt(NewBin(4, 0)))
	for i := 0; i < 16; i += 2 {
		chess16.Set(NewBin(0, uint64(i)))
	}

	assert.True(chess16.FilledAt(NewBin(4, 0)))
	assert.True(chess16.FilledAt(NewBin(2, 3)))

	chess16.Set(NewBin(4, 1))
	assert.True(chess16.FilledAt(NewBin(5, 0)))
}

func TestStaircase(t *testing.T) {
	assert := assert.New(t)
	const TOPLAYR = 44
	staircase := New()
	for i := 0; i < TOPLAYR; i++ {
		staircase.Set(NewBin(uint64(i), 1))
	}

	assert.False(staircase.FilledAt(NewBin(TOPLAYR, 0)))
	assert.False(staircase.EmptyAt(NewBin(TOPLAYR, 0)))

	staircase.Set(NewBin(0, 0))
	assert.True(staircase.FilledAt(NewBin(TOPLAYR, 0)))
}

func TestHole(t *testing.T) {
	assert := assert.New(t)
	hole := New()
	hole.Set(NewBin(8, 0))
	hole.Reset(NewBin(6, 1))
	hole.Reset(NewBin(6, 2))

	assert.True(hole.FilledAt(NewBin(6, 0)))
	assert.True(hole.FilledAt(NewBin(6, 3)))
	assert.False(hole.FilledAt(NewBin(8, 0)))
	assert.False(hole.EmptyAt(NewBin(8, 0)))
	assert.True(hole.EmptyAt(NewBin(6, 1)))
}

func TestFind(t *testing.T) {
	assert := assert.New(t)
	hole := New()
	hole.Set(NewBin(4, 0))
	hole.Reset(NewBin(1, 1))
	hole.Reset(NewBin(0, 7))
	assert.Equal(NewBin(0, 2), hole.FindEmpty().BaseLeft())
}

func TestAlloc(t *testing.T) {
	assert := assert.New(t)
	b := New()
	b.Set(NewBin(1, 0))
	b.Set(NewBin(1, 1))
	b.Reset(NewBin(1, 0))
	b.Reset(NewBin(1, 1))

	assert.Equal(1, b.allocCount)
}

func TestCover(t *testing.T) {
	assert := assert.New(t)
	b := New()
	b.Set(NewBin(2, 0))
	b.Set(NewBin(4, 1))

	assert.Equal(NewBin(4, 1), b.Cover(NewBin(0, 30)))
	assert.Equal(NewBin(2, 0), b.Cover(NewBin(0, 3)))
	assert.Equal(NewBin(2, 0), b.Cover(NewBin(2, 0)))
}

func TestSeqLength(t *testing.T) {
	assert := assert.New(t)
	b := New()
	b.Set(NewBin(3, 0))
	b.Set(NewBin(1, 4))
	b.Set(NewBin(0, 10))
	b.Set(NewBin(3, 2))
	assert.Equal(11, int(b.FindEmpty().BaseOffset()))
}

func TestEmptyFilled(t *testing.T) {
	assert := assert.New(t)
	b := New()

	assert.True(b.EmptyAt(All))
	b.Set(NewBin(1, 0))
	b.Set(NewBin(0, 2))
	b.Set(NewBin(0, 6))
	b.Set(NewBin(1, 5))
	b.Set(NewBin(0, 9))

	assert.False(b.EmptyAt(All))
	assert.True(b.EmptyAt(NewBin(2, 3)))
	assert.False(b.FilledAt(NewBin(2, 3)))
	assert.True(b.FilledAt(NewBin(1, 0)))
	assert.True(b.FilledAt(NewBin(1, 5)))
	assert.False(b.FilledAt(NewBin(1, 3)))

	b.Set(NewBin(0, 3))
	b.Set(NewBin(0, 7))
	b.Set(NewBin(0, 8))

	assert.True(b.FilledAt(NewBin(2, 0)))
	assert.True(b.FilledAt(NewBin(2, 2)))
	assert.False(b.FilledAt(NewBin(2, 1)))

	b.Set(NewBin(1, 2))
	assert.True(b.FilledAt(NewBin(2, 1)))
}

func TestFindEmptyAfter(t *testing.T) {
	assert := assert.New(t)
	hole := New()

	for s := 0; s < 8; s++ {
		for i := s; i < 8; i++ {
			hole.Set(NewBin(3, 0))
			hole.Reset(NewBin(0, uint64(i)))
			f := hole.FindEmptyAfter(NewBin(0, uint64(s)))
			assert.Equal(NewBin(0, uint64(i)), f)
		}
	}
}

func TestFindEmptyAfter2(t *testing.T) {
	cases := []struct {
		filled   []Bin
		bin      Bin
		expected Bin
	}{
		{
			filled:   []Bin{2, 5},
			bin:      2,
			expected: 8,
		},
		{
			filled:   []Bin{255},
			bin:      8191,
			expected: 767,
		},
	}

	for _, c := range cases {
		c := c
		t.Run("", func(t *testing.T) {
			m := New()
			for _, b := range c.filled {
				m.Set(b)
			}

			assert.Equal(t, c.expected, m.FindEmptyAfter(c.bin))
		})
	}

}

func TestFindEmptyAfterSanity(t *testing.T) {
	m := New()
	rand.Seed(4)

	n := 1e5
	for i := 0; i < 1e5; i++ {
		if rand.Float32() < 0.5 {
			m.Set(Bin(rand.Intn(int(n))))
		} else {
			m.Reset(Bin(rand.Intn(int(n))))
		}

		b := Bin(rand.Intn(int(n)))
		next := m.FindEmptyAfter(b)
		if !next.IsNone() && !m.EmptyAt(next) {
			t.Fatalf("next empty after %s bin %s should be empty", b, next)
		}
		if next < b.BaseLeft() {
			t.Fatalf("next empty after %s bin %s should be greater than or equal to %d", b, next, b)
		}
	}
}

func TestFindFilled1(t *testing.T) {
	assert := assert.New(t)
	hole := New()

	hole.Set(NewBin(3, 0))
	hole.Reset(NewBin(0, 0))
	assert.Equal(NewBin(0, 1), hole.FindFilled())
}

func TestFindFilled2(t *testing.T) {
	assert := assert.New(t)
	hole := New()

	hole.Set(NewBin(3, 0))
	hole.Reset(NewBin(0, 1))
	assert.Equal(NewBin(0, 0), hole.FindFilled())
}

func TestFindFilled3(t *testing.T) {
	assert := assert.New(t)
	hole := New()

	hole.Set(NewBin(3, 0))
	hole.Reset(NewBin(2, 0))
	assert.Equal(NewBin(0, 4), hole.FindFilled().BaseLeft())
}

func TestFindFilledAfter(t *testing.T) {
	cases := []struct {
		filled   []Bin
		bin      Bin
		expected Bin
	}{
		{
			filled:   []Bin{1, 4, 9},
			bin:      6,
			expected: 8,
		},
		{
			filled:   []Bin{2047},
			bin:      4014,
			expected: 4014,
		},
		{
			filled:   []Bin{2447, 2471, 2481, 2511, 2535, 2545},
			bin:      2484,
			expected: 2496,
		},
		{
			filled:   []Bin{62218, 62282},
			bin:      62220,
			expected: 62282,
		},
		{
			filled:   []Bin{83156},
			bin:      41012,
			expected: 83156,
		},
		{
			filled:   []Bin{27922, 33531},
			bin:      33462,
			expected: 33528,
		},
	}

	for _, c := range cases {
		c := c
		t.Run("", func(t *testing.T) {
			m := New()
			for _, b := range c.filled {
				m.Set(b)
			}

			assert.Equal(t, c.expected, m.FindFilledAfter(c.bin))
		})
	}
}

func TestFindFilledAfterSanity(t *testing.T) {
	m := New()
	rand.Seed(4)

	n := 1e5
	for i := 0; i < 1e5; i++ {
		if rand.Float32() < 0.5 {
			m.Set(Bin(rand.Intn(int(n))))
		} else {
			m.Reset(Bin(rand.Intn(int(n))))
		}

		b := Bin(rand.Intn(int(n)))
		next := m.FindFilledAfter(b)
		if !next.IsNone() && !m.FilledAt(next) {
			t.Fatalf("next filled after %s bin %s should be filled", b, next)
		}
		if next < b.BaseLeft() {
			t.Fatalf("next empty after %s bin %s should be greater than or equal to %d", b, next, b)
		}
	}
}

var BenchmarkFindFilledAfterRes Bin

func BenchmarkFindFilledAfter(b *testing.B) {
	m := New()
	rand.Seed(4)

	n := 1e5
	for i := .0; i < n; i++ {
		if p := i / n; rand.Float64() < p*p {
			m.Set(Bin(i))
		}
	}

	b.ResetTimer()

	var res Bin
	for i := 0; i < b.N; i++ {
		res = m.FindFilledAfter(Bin(rand.Intn(int(n))))
	}
	BenchmarkFindFilledAfterRes = res
}

func TestFillBefore(t *testing.T) {
	for i := Bin(2); i < 32; i += 2 {
		m := New()
		m.FillBefore(i)
		for b := Bin(0); b < i; b += 2 {
			assert.True(t, m.FilledAt(b))
		}
		assert.False(t, m.FilledAt(Bin(i)))
	}
}
