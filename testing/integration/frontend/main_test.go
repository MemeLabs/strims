// +build integration

package frontend

import (
	"os"
	"testing"

	"github.com/MemeLabs/go-ppspp/testing/driver"
)

func TestMain(m *testing.M) {

	conf := driver.Config{
		SrvAddr: "",
		VpnAddr: "",
	}

	go func() {
		td := driver.Setup(conf)
		defer td.Teardown()
	}()

	os.Exit(m.Run())
}
