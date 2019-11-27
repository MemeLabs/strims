package binmap

import "math"

// table of bins to bitmap masks
var binBitmaps = [...]bitmap{
	0x00000001, 0x00000003, 0x00000002, 0x0000000f,
	0x00000004, 0x0000000c, 0x00000008, 0x000000ff,
	0x00000010, 0x00000030, 0x00000020, 0x000000f0,
	0x00000040, 0x000000c0, 0x00000080, 0x0000ffff,
	0x00000100, 0x00000300, 0x00000200, 0x00000f00,
	0x00000400, 0x00000c00, 0x00000800, 0x0000ff00,
	0x00001000, 0x00003000, 0x00002000, 0x0000f000,
	0x00004000, 0x0000c000, 0x00008000, 0xffffffff,
	0x00010000, 0x00030000, 0x00020000, 0x000f0000,
	0x00040000, 0x000c0000, 0x00080000, 0x00ff0000,
	0x00100000, 0x00300000, 0x00200000, 0x00f00000,
	0x00400000, 0x00c00000, 0x00800000, 0xffff0000,
	0x01000000, 0x03000000, 0x02000000, 0x0f000000,
	0x04000000, 0x0c000000, 0x08000000, 0xff000000,
	0x10000000, 0x30000000, 0x20000000, 0xf0000000,
	0x40000000, 0xc0000000, 0x80000000, 0xffffffff,
}

/**
 * table of tail encoded bytes to bin offsets
 *
 *    high level bins              low level bins
 *
 *          14                            6
 *     12        13                  4         5
 *  10   10    10   10           2     2     2    2
 * 8  9 8  9  8  9 8  11       0  1  0  1  0  1  0  3
 *
 */
var bitmapBins = [...]uint8{
	255, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3,
	8, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3,
	10, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3,
	9, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3,
	12, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3,
	8, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3,
	10, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3,
	9, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3,
	14, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3,
	8, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3,
	10, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3,
	9, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3,
	13, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3,
	8, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3,
	10, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 3,
	11, 0, 2, 1, 4, 0, 2, 1, 6, 0, 2, 1, 5, 0, 2, 7,
}

const (
	bitmapLayerBits = Bin(63)
	bitmapFilled    = bitmap(math.MaxUint32)
	bitmapEmpty     = bitmap(0)
)

type bitmap uint32

func (b bitmap) Reset() bitmap {
	return 0
}

func (b bitmap) Set(i uint32, v bool) bitmap {
	if v {
		return b | (1 << i)
	}
	return b &^ (1 << i)
}

func (b bitmap) Get(i uint32) bool {
	return b&(1<<i) != 0
}

func (b bitmap) Empty() bool {
	return b == bitmapEmpty
}

func (b bitmap) Filled() bool {
	return b == bitmapFilled
}

func bitmapBin(b bitmap) Bin {
	t := bitmapBins[b&0xff]
	if t < 16 {
		if t != 7 {
			return Bin(t)
		}

		b++
		b &= -b
		if b == 0 {
			return bitmapLayerBits / 2
		}
		if (b & 0xffff) == 0 {
			return 15
		}
		return 7
	}

	b >>= 8
	t = bitmapBins[b&0xff]
	if t < 16 {
		return Bin(16 + t)
	}

	b >>= 8
	t = bitmapBins[b&0xff]
	if t < 16 {
		if t != 7 {
			return Bin(32 + t)
		}

		b++
		b &= -b
		if (0xffff & b) == 0 {
			return 47
		}
		return 39
	}

	b >>= 8
	return Bin(48 + bitmapBins[b&0xff])
}

func offsetBitmapBin(b Bin, bm bitmap) Bin {
	if bm.Filled() {
		return b
	}

	return b.BaseLeft() + bitmapBin(bm)
}
