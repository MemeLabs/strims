package pool

// DefaultPool ...
var DefaultPool = New(10)

// Get ...
func Get(size int) *[]byte {
	return DefaultPool.Get(size)
}

// Put ...
func Put(b *[]byte) {
	DefaultPool.Put(b)
}
