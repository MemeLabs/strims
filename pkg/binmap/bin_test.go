package binmap

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestInitGet(t *testing.T) {
	assert := assert.New(t)

	assert.Equal(0x1, int(NewBin(1, 0)))
	assert.Equal(0xB, int(NewBin(2, 1)))
	assert.Equal(uint64(2), NewBin(2, 1).Layer())
	assert.Equal(uint64(34), NewBin(34, 2345).Layer())
	assert.Equal(0x7ffffffff, int(NewBin(34, 2345).LayerBits()))
	assert.Equal(uint64(1), NewBin(2, 1).LayerOffset())
	assert.Equal(uint64(2345), NewBin(34, 2345).LayerOffset())
	assert.Equal((1<<1)-1, int(NewBin(0, 123).LayerBits()))
	assert.Equal((1<<17)-1, int(NewBin(16, 123).LayerBits()))
}

func TestNavigation(t *testing.T) {
	assert := assert.New(t)
	mid := NewBin(4, 18)
	assert.Equal(NewBin(5, 9), mid.Parent())
	assert.Equal(NewBin(3, 36), mid.Left())
	assert.Equal(NewBin(3, 37), mid.Right())
	assert.Equal(NewBin(5, 9), NewBin(4, 19).Parent())
	up32 := NewBin(30, 1)
	assert.Equal(NewBin(31, 0), up32.Parent())
}

func TestOverflows(t *testing.T) {
	assert := assert.New(t)
	assert.False(None.Contains(NewBin(0, 1)))
	assert.True(All.Contains(NewBin(0, 1)))
	assert.Equal(uint64(0), None.BaseLength())
}

func TestAdvanced(t *testing.T) {
	assert := assert.New(t)
	assert.Equal(uint64(4), NewBin(2, 3).BaseLength())
	assert.False(NewBin(1, 1234).IsBase())
	assert.True(NewBin(0, 12345).IsBase())
	assert.Equal(NewBin(0, 2), NewBin(1, 1).BaseLeft())
	assert.Equal(NewBin(0, 3), NewBin(1, 1).BaseRight())
}

func TestBinToString(t *testing.T) {
	assert := assert.New(t)

	assert.Equal(All.String(), "ALL")
	assert.Equal(None.String(), "NONE")
	assert.Equal(NewBin(0, 1).String(), "2")
}

func TestLayerLeftRight(t *testing.T) {
	for l := uint64(0); l < 10; l++ {
		for o := uint64(0); l < 10; l++ {
			left := NewBin(l, o)
			right := NewBin(l, o+1)
			assert.Equal(t, right, left.LayerRight())
			assert.Equal(t, left, right.LayerLeft())
		}

		assert.Equal(t, NewBin(l, 0).LayerLeft(), None)
	}
}

func TestLayerShift(t *testing.T) {
	for i := uint64(0); i < 10; i++ {
		for j := uint64(4); j < 8; j++ {
			assert.Equal(t, NewBin(j, i).LayerShift(j-1), NewBin(j-1, i*2))
			assert.Equal(t, NewBin(j, i).LayerShift(j-2), NewBin(j-2, i*4))
			assert.Equal(t, NewBin(j, i).LayerShift(j-3), NewBin(j-3, i*8))
			assert.Equal(t, NewBin(j, i).LayerShift(j-4), NewBin(j-4, i*16))

			assert.Equal(t, NewBin(j-1, i*2).LayerShift(j), NewBin(j, i))
			assert.Equal(t, NewBin(j-2, i*4).LayerShift(j), NewBin(j, i))
			assert.Equal(t, NewBin(j-3, i*8).LayerShift(j), NewBin(j, i))
			assert.Equal(t, NewBin(j-4, i*16).LayerShift(j), NewBin(j, i))
		}
	}
}
