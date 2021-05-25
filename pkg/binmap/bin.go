package binmap

import (
	"math/bits"
	"strconv"
)

/**
 * Numbering for (aligned) logarithmic bins.
 *
 * Each number stands for an interval
 *   [layer_offset * 2^layer, (layer_offset + 1) * 2^layer).
 *
 * The following value is called as base_offset:
 *   layer_offset * 2^layer -- is called
 *
 * Bin numbers in the tail111 encoding: meaningless bits in
 * the tail are set to 0111...11, while the head denotes the offset.
 * bin = 2 ^ (layer + 1) * layer_offset + 2 ^ layer - 1
 *
 * Thus, 1101 is the bin at layer 1, offset 3 (i.e. fourth).
 */

/**
 *
 *                     +-----------------0111-----------------+
 *                    |                                       |
 *          +-------0011-------+                    +-------1011-------+
 *         |                   |                   |                   |
 *    +--0001--+          +--0101--+          +--1001--+          +--1101--+
 *   |         |         |         |         |         |         |         |
 * 0000      0010      0100      0110      1000      1010      1100      1110
 *
 *
 *
 *               7
 *           /       \
 *       3              11
 *     /   \           /  \
 *   1       5       9     13
 *  / \     / \     / \    / \
 * 0   2   4   6   8  10  12 14
 *
 */

// min/max bins
const (
	None = Bin(0xffffffffffffffff)
	All  = Bin(0x7fffffffffffffff)
)

// Bin ...
type Bin uint64

// NewBin ...
func NewBin(layer, offset uint64) Bin {
	if layer <= uint64(bitmapLayerBits) {
		return Bin((2*offset+1)<<layer) - 1
	}
	return None
}

// String ...
func (b Bin) String() string {
	if b == All {
		return "ALL"
	} else if b == None {
		return "NONE"
	}
	return strconv.FormatUint(uint64(b), 10)
}

// IsAll ...
func (b Bin) IsAll() bool {
	return b == All
}

// IsNone ...
func (b Bin) IsNone() bool {
	return b == None
}

// LayerBits ...
func (b Bin) LayerBits() Bin {
	return b ^ (b + 1)
}

// Parent ...
func (b Bin) Parent() Bin {
	lbs := b.LayerBits()
	nlbs := ^(lbs + 1)
	return (b | lbs) & nlbs
}

// Left descendent of bin
func (b Bin) Left() Bin {
	t := b + 1
	return b ^ ((t & -t) >> 1)
}

// Right descendent of bin
func (b Bin) Right() Bin {
	t := b + 1
	return b + ((t & -t) >> 1)
}

// BaseLeft leftmost base bin in b
func (b Bin) BaseLeft() Bin {
	if b == None {
		return None
	}
	return b & (b + 1)
}

// BaseRight rightmost base bin in b
func (b Bin) BaseRight() Bin {
	if b == None {
		return None
	}
	return (b | (b + 1)) - 1
}

// LayerLeft bin at layer offset - 1 or None
func (b Bin) LayerLeft() Bin {
	if b == None {
		return None
	}
	t := b.LayerBits() + 1
	if b < t {
		return None
	}
	return b - t
}

// LayerRight bin at layer offset + 1
func (b Bin) LayerRight() Bin {
	if b == None {
		return None
	}
	return b + b.LayerBits() + 1
}

// Base true if b is in the base
func (b Bin) Base() bool {
	return b&1 == 0
}

// LayerOffset index in layer
func (b Bin) LayerOffset() uint64 {
	return uint64(b >> (b.Layer() + 1))
}

// LayerShift leftmost bin above or below b at layer z
func (b Bin) LayerShift(z uint64) Bin {
	return b&^b.LayerBits() | (1 << z) - 1
}

// Contains true if o is equal to or a descendent of b
func (b Bin) Contains(o Bin) bool {
	if b == None {
		return false
	}
	return (b&(b+1)) <= o && o < (b|(b+1))
}

// Layer tree height at b
func (b Bin) Layer() uint64 {
	return uint64(bits.TrailingZeros64(uint64(b + 1)))
}

// BaseOffset index of leftmost bin in layer 0
func (b Bin) BaseOffset() uint64 {
	return uint64(b&(b+1)) >> 1
}

// BaseLength width of base layer
func (b Bin) BaseLength() uint64 {
	t := b + 1
	return uint64(t & -t)
}

// Sibling ...
func (b Bin) Sibling() Bin {
	return b ^ (b.LayerBits() + 1)
}

// IsLeft ...
func (b Bin) IsLeft() bool {
	return b&(b.LayerBits()+1) == 0
}

// IsRight ...
func (b Bin) IsRight() bool {
	return !b.IsLeft()
}
