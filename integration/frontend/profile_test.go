package frontend

import (
	"context"
	"fmt"
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/integration/driver"
	"github.com/MemeLabs/go-ppspp/pkg/api"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/stretchr/testify/assert"
)

func TestCreateProfile(t *testing.T) {
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

	client := api.NewProfileClient(td.Client(&driver.ClientOptions{}))
	for scenario, tc := range tcs {
		t.Run(scenario, func(t *testing.T) {
			assert := assert.New(t)

			ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
			defer cancel()

			res := &pb.CreateProfileResponse{}
			err := client.Create(ctx, tc.req, res)
			if err == nil {
				assert.Equal(tc.expected.res.GetProfile().GetName(), res.GetProfile().GetName())
			} else {
				assert.Equal(tc.expected.res, res)
				assert.Equal(tc.expected.err, err)
			}
		})
	}
}
