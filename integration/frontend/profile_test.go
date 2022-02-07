package frontend

import (
	"context"
	"fmt"
	"testing"
	"time"

	"github.com/MemeLabs/go-ppspp/integration/driver"
	authv1 "github.com/MemeLabs/go-ppspp/pkg/apis/auth/v1"
	profilev1 "github.com/MemeLabs/go-ppspp/pkg/apis/profile/v1"
	"github.com/stretchr/testify/assert"
)

func TestSignUp(t *testing.T) {
	type expectation struct {
		res *authv1.SignUpResponse
		err error
	}

	tcs := map[string]struct {
		req      *authv1.SignUpRequest
		expected expectation
	}{
		"success": {
			req: &authv1.SignUpRequest{
				Name:     "jbpratt",
				Password: "ilovemajora",
			},
			expected: expectation{
				res: &authv1.SignUpResponse{
					Profile: &profilev1.Profile{
						Name: "jbpratt",
					},
				},
				err: nil,
			},
		},
		"duplicate username": {
			req: &authv1.SignUpRequest{
				Name:     "jbpratt",
				Password: "ilovemajora",
			},
			expected: expectation{
				res: &authv1.SignUpResponse{},
				err: fmt.Errorf("username already taken"),
			},
		},
	}

	client := authv1.NewAuthFrontendClient(td.Client(&driver.ClientOptions{}))
	for scenario, tc := range tcs {
		t.Run(scenario, func(t *testing.T) {
			assert := assert.New(t)

			ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
			defer cancel()

			res := &authv1.SignUpResponse{}
			err := client.SignUp(ctx, tc.req, res)
			if err == nil {
				assert.Equal(tc.expected.res.GetProfile().GetName(), res.GetProfile().GetName())
			} else {
				assert.Equal(tc.expected.res, res)
				assert.Equal(tc.expected.err, err)
			}
		})
	}
}
