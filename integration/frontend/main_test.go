package frontend

import (
	"log"
	"os"
	"testing"

	"github.com/MemeLabs/go-ppspp/integration/driver"
)

var td driver.Driver

func TestMain(m *testing.M) {
	var err error
	td, err = NewDriver()
	if err != nil {
		log.Fatal(err)
	}

	code := m.Run()

	td.Close()

	os.Exit(code)
}
