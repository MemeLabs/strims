package ioutil

import "io"

var zeros = make([]byte, 32*1024)

func WriteZerosN(w io.Writer, n int64) (int64, error) {
	for wn := n; wn > 0; {
		b := zeros
		if wn < int64(len(b)) {
			b = b[:wn]
		}

		nn, err := w.Write(b)
		wn -= int64(nn)
		if err != nil {
			return n - wn, err
		}
	}
	return n, nil
}
