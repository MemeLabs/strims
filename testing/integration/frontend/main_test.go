// +build integration

package frontend

import (
	"context"
	"os"
	"testing"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/MemeLabs/go-ppspp/testing/driver"
)

var td *driver.TestDriver

func TestMain(m *testing.M) {

	conf := driver.Config{
		SrvAddr: "localhost:6060",
		VpnAddr: "0.0.0.0:8082",
	}

	td = driver.Setup(conf)
	defer td.Teardown()

	os.Exit(m.Run())
}

func TestCreateProfile(t *testing.T) {
	t.Parallel()

	req := &pb.CreateProfileRequest{
		Name:     "jbpratt",
		Password: "ilovemajora",
	}

	if err := td.Client.Call(context.Background(), "createProfile", req); err != nil {
		t.Error(err)
	}
}
