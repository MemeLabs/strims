package prefixstream

import (
	"bytes"
	"io"
	"testing"
)

func TestE2E(t *testing.T) {
	b := bytes.NewBuffer(nil)
	w := NewWriter(b)
	r := NewReader(b)

	ns := []int{27, 100000, 128}

	for _, n := range ns {
		w.Write(make([]byte, n))
	}

	for _, n := range ns {
		rn, err := r.Read(make([]byte, n))
		if err != io.EOF || n != rn {
			t.Errorf("expected to read %d, read %d", n, rn)
			t.FailNow()
		}
	}
}
