package binmap

import (
	"log"
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
	assert.False(NewBin(1, 1234).Base())
	assert.True(NewBin(0, 12345).Base())
	assert.Equal(NewBin(0, 2), NewBin(1, 1).BaseLeft())
	assert.Equal(NewBin(0, 3), NewBin(1, 1).BaseRight())
}

func TestBinToString(t *testing.T) {
	assert := assert.New(t)

	assert.Equal(All.String(), "ALL")
	assert.Equal(None.String(), "NONE")
	assert.Equal(NewBin(0, 1).String(), "2")
}

func TestMeme(t *testing.T) {
	log.Println(NewBin(2, 1).LayerBits())
}
