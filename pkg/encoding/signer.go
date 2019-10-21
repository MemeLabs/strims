package encoding

// TODO: content integrity

// func NewSigner(s crypto.Signer, h crypto.Hash) Signer {
// 	return Signer{s, h}
// }

// type Signer struct {
// 	s crypto.Signer
// 	h crypto.Hash
// }

// Sign ...
// func (s Signer) Sign(d []byte) ([]byte, error) {
// 	return s.s.Sign(rand.Reader, d, s.h)
// }

// type DebugWriter struct {
// 	chunkSize          int
// 	chunksPerSignature int
// 	s                  Signer
// }

// func NewDebugWriter(chunkSize, chunksPerSignature int, s Signer) *DebugWriter {
// 	return &DebugWriter{
// 		chunkSize,
// 		chunksPerSignature,
// 		s,
// 	}
// }

// Write ...
// func (m *DebugWriter) Write(p []byte) (n int, err error) {
// 	chunks := make([][]byte, m.chunksPerSignature)
// 	signatures := make([][]byte, m.chunksPerSignature)
// 	for i := 0; i < m.chunksPerSignature; i++ {
// 		off := i * m.chunkSize
// 		chunks[i] = p[off : off+m.chunkSize]
// 		signatures[i], err = m.s.Sign(chunks[i])
// 		if err != nil {
// 			return
// 		}
// 	}

// 	spew.Dump(signatures)

// 	n = len(p)
// 	return
// }

// func generateKey() {

// 	msg := "hello, world"
// 	hash := sha256.Sum256([]byte(msg))

// 	r, s, err := ecdsa.Sign(rand.Reader, privateKey, hash[:])
// 	if err != nil {
// 		panic(err)
// 	}
// 	fmt.Printf("signature: (0x%x, 0x%x)\n", r, s)

// 	valid := ecdsa.Verify(&privateKey.PublicKey, hash[:], r, s)
// 	fmt.Println("signature verified:", valid)
// }
