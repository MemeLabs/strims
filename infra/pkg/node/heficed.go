package node

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io/ioutil"
	"net/http"
	"time"

	"golang.org/x/crypto/ssh"
	"golang.org/x/oauth2"
	"golang.org/x/oauth2/clientcredentials"
)

const heficedAPIEndpoint string = "https://api.heficed.com/"

type HeficedDriver struct {
	client   *http.Client
	tenantID string
}

func NewHeficedDriver(clientID, clientSecret, tenantID string) (*HeficedDriver, error) {
	ctx := context.Background()

	conf := clientcredentials.Config{
		ClientID:     clientID,
		ClientSecret: clientSecret,
		Scopes:       []string{"kronoscloud", "sshkeys"},
		TokenURL:     "https://iam-proxy.heficed.com/oauth2/token",
	}

	tok, err := conf.Token(ctx)
	if err != nil {
		return nil, err
	}

	client := oauth2.NewClient(ctx, oauth2.StaticTokenSource(tok))
	fmt.Println(tok)

	if len(tenantID) == 0 {
		// TODO: fetch tenantID
		resp, err := client.Get(heficedAPIEndpoint)
		if err != nil {
			return nil, err
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusOK {
			return nil, fmt.Errorf("request failed with: %v", resp)
		}

		body, err := ioutil.ReadAll(resp.Body)
		if err != nil {
			return nil, err
		}

		var tenants heficedTenants
		if err := json.Unmarshal(body, &tenants); err != nil {
			return nil, err
		}

		for _, x := range tenants.Data {
			for _, y := range x {
				fmt.Println(y.ID)
			}
		}
	}
	return &HeficedDriver{client, tenantID}, nil
}

func (d *HeficedDriver) Provider() string {
	return "heficed"
}

func (d *HeficedDriver) Regions(ctx context.Context, req *RegionsRequest) ([]*Region, error) {
	return nil, fmt.Errorf("unimplemented")
}

func (d *HeficedDriver) SKUs(ctx context.Context, req *SKUsRequest) ([]*SKU, error) {
	return nil, fmt.Errorf("unimplemented")
}

func (d *HeficedDriver) Create(ctx context.Context, req *CreateRequest) (*Node, error) {
	return nil, fmt.Errorf("unimplemented")
}

func (d *HeficedDriver) List(ctx context.Context, req *ListRequest) ([]*Node, error) {
	return nil, fmt.Errorf("unimplemented")
}

func (d *HeficedDriver) Delete(ctx context.Context, req *DeleteRequest) error {
	return fmt.Errorf("unimplemented")
}

func (d *HeficedDriver) findOrAddSSHKey(ctx context.Context, public string) (string, error) {
	path := fmt.Sprintf("%s/%s/sshkeys", heficedAPIEndpoint, d.tenantID)

	var keys heficedSSHKeys
	resp, err := d.client.Get(path)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return "", fmt.Errorf("request failed with: %v", resp)
	}

	body, err := ioutil.ReadAll(resp.Body)
	if err != nil {
		return "", err
	}

	if err := json.Unmarshal(body, &keys); err != nil {
		return "", err
	}

	pk, err := ssh.ParsePublicKey([]byte(public))
	if err != nil {
		return "", err
	}

	f := ssh.FingerprintLegacyMD5(pk)

	for _, x := range keys.Data {
		for _, y := range x {
			if y.FingerPrint == f {
				return y.ID, nil
			}
		}
	}

	data := map[string]string{
		"publicKey": public,
		"label":     fmt.Sprintf("infra-key-%d", time.Now().UnixNano()),
	}

	z, err := json.Marshal(data)
	if err != nil {
		return "", err
	}

	var out map[string]string
	resp, err = d.client.Post(path, "application/json", bytes.NewBuffer(z))
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusCreated {
		return "", fmt.Errorf("request failed with: %v", resp)
	}

	body, err = ioutil.ReadAll(resp.Body)
	if err != nil {
		return "", err
	}

	if err := json.Unmarshal(body, &out); err != nil {
		return "", err
	}

	return out["id"], nil
}

type heficedSSHKeys struct {
	Data [][]struct {
		ID          string `json:"id"`
		Label       string `json:"label"`
		FingerPrint string `json:"fingerPrint"`
		Created     int    `json:"created"`
	} `json:"data"`
}

type heficedTenants struct {
	Data [][]struct {
		ID string `json:"id"`
	} `json:"data"`
}
