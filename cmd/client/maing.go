package main

import (
	"github.com/MemeLabs/go-ppspp/pkg/p2p"
)

func main() {
	p2p.NewWS()

	select {}
}
