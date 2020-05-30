package hmac_drbg

import (
	"crypto/md5"
	"crypto/sha1"
	"crypto/sha256"
	"crypto/sha512"
	"hash"
	"math"
	"testing"

	"github.com/davecgh/go-spew/spew"
	"github.com/minio/blake2b-simd"
	"golang.org/x/crypto/sha3"
)

func TestRNG(t *testing.T) {
	cases := []struct {
		Seed  []byte
		Hash  func() hash.Hash
		Value uint64
	}{
		{
			Seed:  []byte{0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff},
			Hash:  sha1.New,
			Value: 13830991175589696070,
		},
		{
			Seed:  []byte{0xfe, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff},
			Hash:  sha512.New,
			Value: 14618493976459815342,
		},
		{
			Seed:  []byte{0xfd, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff},
			Hash:  blake2b.New512,
			Value: 8389461046711400686,
		},
	}

	for _, c := range cases {
		rng := NewRNG(c.Hash, c.Seed)
		if v := rng.Uint64(); v != c.Value {
			t.Errorf("unexpected value for test seed: got %d expected %d", v, c.Value)
		}
	}
}

func TestRNGDistribution(t *testing.T) {
	t.SkipNow()

	const (
		n             = 1000000
		binCount      = 1000
		targetBinSize = n / binCount
	)

	seed := []byte{0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff}
	rng := NewRNG(blake2b.New512, seed)

	bins := make([]int, binCount)
	for i := 0; i < n; i++ {
		bins[rng.Uint32()%uint32(binCount)]++
	}

	var e float64
	for _, v := range bins {
		a := float64(v - targetBinSize)
		e += a * a
	}

	rmse := math.Sqrt(e / float64(binCount))
	spew.Dump(rmse)
}

func BenchmarkMd5(b *testing.B) {
	runHashBenchmark(b, md5.New)
}
func BenchmarkSha1(b *testing.B) {
	runHashBenchmark(b, sha1.New)
}
func BenchmarkSha224(b *testing.B) {
	runHashBenchmark(b, sha256.New224)
}
func BenchmarkSha256(b *testing.B) {
	runHashBenchmark(b, sha256.New)
}
func BenchmarkSha384(b *testing.B) {
	runHashBenchmark(b, sha512.New384)
}
func BenchmarkSha512(b *testing.B) {
	runHashBenchmark(b, sha512.New)
}
func BenchmarkSha3_224(b *testing.B) {
	runHashBenchmark(b, sha3.New224)
}
func BenchmarkSha3_256(b *testing.B) {
	runHashBenchmark(b, sha3.New256)
}
func BenchmarkSha3_384(b *testing.B) {
	runHashBenchmark(b, sha3.New384)
}
func BenchmarkSha3(b *testing.B) {
	runHashBenchmark(b, sha3.New512)
}
func BenchmarkBlake2b256(b *testing.B) {
	runHashBenchmark(b, blake2b.New256)
}
func BenchmarkBlake2b512(b *testing.B) {
	runHashBenchmark(b, blake2b.New512)
}

func runHashBenchmark(b *testing.B, h func() hash.Hash) {
	seed := []byte{0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff}
	rng := NewRNG(h, seed)

	for i := 0; i < b.N; i++ {
		rng.Uint64()
	}
}
