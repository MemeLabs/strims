package ioutil

import (
	"io"

	"github.com/MemeLabs/go-ppspp/pkg/mathutil"
)

func DiscardN(r io.Reader, n int64) (int64, error) {
	return DiscardNBuf(r, n, nil)
}

func DiscardNBuf(r io.Reader, n int64, buf []byte) (int64, error) {
	if buf == nil {
		buf = make([]byte, mathutil.Min(32*1024, n))
	}

	for rn := n; rn > 0; {
		b := buf
		if rn < int64(len(b)) {
			b = b[:rn]
		}

		nn, err := r.Read(b)
		rn -= int64(nn)
		if err != nil {
			return n - rn, err
		}
	}
	return n, nil
}
