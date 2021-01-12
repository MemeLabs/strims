package frontend

import (
	"context"
	"fmt"
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/integration/driver"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/stretchr/testify/assert"
)

func TestCreateProfile(t *testing.T) {
	type expectation struct {
		res *profilev1.CreateProfileResponse
		err error
	}

	tcs := map[string]struct {
		req      *profilev1.CreateProfileRequest
		expected expectation
	}{
		"success": {
			req: &profilev1.CreateProfileRequest{
				Name:     "jbpratt",
				Password: "ilovemajora",
			},
			expected: expectation{
				res: &profilev1.CreateProfileResponse{
					Profile: &profilev1.Profile{
						Name: "jbpratt",
					},
				},
				err: nil,
			},
		},
		"duplicate username": {
			req: &profilev1.CreateProfileRequest{
				Name:     "jbpratt",
				Password: "ilovemajora",
			},
			expected: expectation{
				res: &profilev1.CreateProfileResponse{},
				err: fmt.Errorf("profile name not available"),
			},
		},
	}

	client := profilev1.NewProfileServiceClient(td.Client(&driver.ClientOptions{}))
	for scenario, tc := range tcs {
		t.Run(scenario, func(t *testing.T) {
			assert := assert.New(t)

			ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
			defer cancel()

			res := &profilev1.CreateProfileResponse{}
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
