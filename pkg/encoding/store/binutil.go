package store

import "github.com/MemeLabs/go-ppspp/pkg/binmap"

func binByte(b binmap.Bin, chunkSize uint64) uint64 {
	return uint64(b/2) * chunkSize
}

func byteBin(b, chunkSize uint64) binmap.Bin {
	return binmap.Bin(b*2) / binmap.Bin(chunkSize)
}
