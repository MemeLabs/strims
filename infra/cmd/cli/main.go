package main

import (
	"log"

	"github.com/MemeLabs/go-ppspp/infra/internal/cmd"
)

func main() {
	if err := cmd.Execute(); err != nil {
		log.Fatal(err)
	}
}
