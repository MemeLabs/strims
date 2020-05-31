// +build integration

package frontend

import (
	"context"
	"fmt"
	"os"
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/integration/driver"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/stretchr/testify/assert"
)

var td *driver.TestDriver

func TestMain(m *testing.M) {
	conf := driver.Config{
		VpnAddr: "0.0.0.0:8082",
	}

	td = driver.Setup(conf)
	defer td.Teardown()

	os.Exit(m.Run())
}

func TestCreateProfile(t *testing.T) {
	t.Parallel()

	type expectation struct {
		res *pb.CreateProfileResponse
		err error
	}

	tcs := map[string]struct {
		req      *pb.CreateProfileRequest
		expected expectation
	}{
		"success": {
			req: &pb.CreateProfileRequest{
				Name:     "jbpratt",
				Password: "ilovemajora",
			},
			expected: expectation{
				res: &pb.CreateProfileResponse{
					Profile: &pb.Profile{
						Name: "jbpratt",
					},
				},
				err: nil,
			},
		},
		"duplicate username": {
			req: &pb.CreateProfileRequest{
				Name:     "jbpratt",
				Password: "ilovemajora",
			},
			expected: expectation{
				res: &pb.CreateProfileResponse{},
				err: fmt.Errorf("profile name not available"),
			},
		},
	}

	for scenario, tc := range tcs {
		t.Run(scenario, func(t *testing.T) {
			assert := assert.New(t)

			ctx, cancel := context.WithTimeout(context.Background(), time.Second)
			defer cancel()

			res := &pb.CreateProfileResponse{}
			err := td.Client.CallUnary(ctx, "createProfile", tc.req, res)
			if err == nil {
				assert.Equal(tc.expected.res.GetProfile().GetName(), res.GetProfile().GetName())
			} else {
				assert.Equal(tc.expected.res, res)
				assert.Equal(tc.expected.err, err)
			}
		})
	}
}
