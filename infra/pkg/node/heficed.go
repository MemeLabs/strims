package node

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io/ioutil"
	"net/http"
	"strconv"
	"time"

	"github.com/golang/geo/s2"
	"golang.org/x/crypto/ssh"
	"golang.org/x/oauth2"
	"golang.org/x/oauth2/clientcredentials"
)

const (
	heficedOS                 = "Ubuntu LTS 20.04"
	heficedAPIEndpoint string = "https://api.heficed.com/"
)

var heficedRegions = []*Region{
	{
		Name:   "br-sao1",
		City:   "São Paulo, Brazil",
		LatLng: s2.LatLngFromDegrees(-23.5505, -46.6333),
	},
	{
		Name:   "de-fra1",
		City:   "Frankfurt, Germany",
		LatLng: s2.LatLngFromDegrees(50.1109, 8.6821),
	},
	{
		Name:   "uk-lon1",
		City:   "London, United Kingdom",
		LatLng: s2.LatLngFromDegrees(51.5074, -0.1278),
	},
	{
		Name:   "us-chi1",
		City:   "Chicago, IL, US",
		LatLng: s2.LatLngFromDegrees(41.8781, -87.6298),
	},
	{
		Name:   "us-lax1",
		City:   "Los Angeles, CA, US",
		LatLng: s2.LatLngFromDegrees(34.0522, -118.2437),
	},
	{
		Name:   "za-jhb1",
		City:   "Johannesburg, South Africa",
		LatLng: s2.LatLngFromDegrees(-26.2041, 28.0473),
	},
}

type heficedAlacart struct {
	name                string
	vcpus               int
	memory              int
	disk                int
	additionalBandwidth int
}

var heficedSpecsAlacart = []*heficedAlacart{
	{name: "1210", vcpus: 1, memory: 2 << 10, disk: 10, additionalBandwidth: 0},
	{name: "1420", vcpus: 1, memory: 4 << 10, disk: 20, additionalBandwidth: 0},
	{name: "1840", vcpus: 1, memory: 8 << 10, disk: 40, additionalBandwidth: 0},
	{name: "2840", vcpus: 2, memory: 8 << 10, disk: 40, additionalBandwidth: 0},
	{name: "21660", vcpus: 2, memory: 16 << 10, disk: 60, additionalBandwidth: 10},
}

// HeficedDriver is a node driver which implements the Driver interface for
// the Heficed provider. More information can be found in the API docs.
// https://api.heficed.com/docs/swagger.html
type HeficedDriver struct {
	client   *http.Client
	tenantID string
}

// NewHeficedDriver creates a HeficedDriver from oauth credentials. It uses
// the `oauth2` package and requires ["kronoscloud, "sshkeys"] scope. An error
// is returned if there is a problem creating the oauth token, or there is no
// provided `tenantID`.
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

	if len(tenantID) == 0 {
		// TODO(jbpratt): fetch tenantID. I could not determine the oauth scope
		// needed to fetch this if not provided, but a solution to retrieve
		// the value is provided by Heficed in the API docs.
		return nil, fmt.Errorf("must retrieve TenantID from Heficed console (https://api.heficed.com/docs/swagger.html)")
	}
	return &HeficedDriver{client, tenantID}, nil
}

// Provider returns the driver's provider name
func (d *HeficedDriver) Provider() string {
	return "heficed"
}

// DefaultUser used to log into the instances
func (d *HeficedDriver) DefaultUser() string {
	return "root"
}

// Regions returns a list of available regions for the current credentials
func (d *HeficedDriver) Regions(ctx context.Context, req *RegionsRequest) ([]*Region, error) {
	return append(make([]*Region, 0, len(heficedRegions)), heficedRegions...), nil
}

// SKUs queries a list of templates and creates a set of machine sizes from
// predefined specs in `heficed.go`. The templates are determined by region
// provided in the `SKUsRequest` and quotes for each 'order' built are
// requested and added on to the resulting SKU.
func (d *HeficedDriver) SKUs(ctx context.Context, req *SKUsRequest) ([]*SKU, error) {
	if req.Region == "" {
		return nil, fmt.Errorf("must supply a region")
	}

	path := fmt.Sprintf("%s/common/kronoscloud", heficedAPIEndpoint)
	skus := []*SKU{}

	regions, err := d.Regions(ctx, &RegionsRequest{})
	if err != nil {
		return nil, err
	}

	for _, region := range regions {
		if region.Name != req.Region {
			continue
		}

		templateID, err := d.findTemplateID(ctx, path, req.Region)
		if err != nil {
			return nil, err
		}

		for _, spec := range heficedSpecsAlacart {
			data := toHeficedOrder(spec, templateID, req.Region)
			var out heficedQuote
			if err := d.post(
				ctx,
				fmt.Sprintf("%s/%s", path, "order/quote"),
				&data,
				&out,
			); err != nil {
				return nil, err
			}
			out.Items[0].name = spec.name
			skus = append(skus, heficedSKU(&out))
		}
	}

	return skus, nil
}

func heficedSKU(quote *heficedQuote) *SKU {
	return &SKU{
		Name:         quote.Items[0].name,
		CPUs:         quote.Items[0].Configuration.Raw.Vcpu,
		Memory:       quote.Items[0].Configuration.Raw.Memory,
		Disk:         quote.Items[0].Configuration.Raw.Disk,
		NetworkCap:   int(1000 * (quote.Items[1].Total / 0.6)),
		NetworkSpeed: 10000,
		PriceHourly: &Price{
			Value:    quote.UsageBasedRate,
			Currency: quote.Currency,
		},
		PriceMonthly: &Price{
			// machine total + Connect service
			Value:    quote.Items[0].Total + quote.Items[1].Total,
			Currency: quote.Currency,
		},
	}
}

func toHeficedOrder(spec *heficedAlacart, templateID, region string) *heficedOrder {
	return &heficedOrder{
		InstanceTypeID:      "linux",
		LocationID:          region,
		TemplateID:          templateID,
		Vcpu:                spec.vcpus,
		Memory:              spec.memory,
		Disk:                spec.disk,
		BillingTypeID:       1, // monthly
		AdditionalBandwidth: spec.additionalBandwidth,
		UseCredit:           true,
	}
}

func heficedNode(instance *heficedInstance) *Node {
	region := &Region{}
	for _, reg := range heficedRegions {
		if reg.Name == instance.Location.Name {
			region = reg
		}
	}

	v4 := []string{instance.Network.V4.Ipaddress}
	v4 = append(v4, instance.Network.V4.AdditionalIps...)
	v6 := []string{instance.Network.V6.Ipaddress}
	v6 = append(v6, instance.Network.V6.AdditionalIps...)
	return &Node{
		ProviderID: strconv.Itoa(instance.ID),
		Name:       instance.Hostname,
		Memory:     instance.Memory,
		CPUs:       instance.Vcpu,
		Networks: &Networks{
			V4: v4,
			V6: v6,
		},
		Status: instance.Status == "running" || instance.Status == "pending",
		SKU:    nil,
		Region: region,
	}
}

// Create provisions a new node based on the name which matches the spec
// definition in `heficedSpecsAlacart`. If the ssh public key provided in the
// request is not already existing, a new key entry is added. The order is then
// placed and waits for the machine to have an "running" status. An error is
// returned if there is a problem finding the template, finding or adding the
// ssh key, or placing the order.
func (d *HeficedDriver) Create(ctx context.Context, req *CreateRequest) (*Node, error) {
	path := fmt.Sprintf("%s/common/kronoscloud", heficedAPIEndpoint)
	var order *heficedOrder
	for _, spec := range heficedSpecsAlacart {
		if req.SKU == spec.name {
			templateID, err := d.findTemplateID(ctx, path, req.Region)
			if err != nil {
				return nil, err
			}
			order = toHeficedOrder(spec, templateID, req.Region)
		}
	}

	if req.BillingType == Hourly {
		order.BillingTypeID = -1 // hourly
	}

	if order == nil {
		return nil, fmt.Errorf("unable to find name of sku")
	}

	sshKeyID, err := d.findOrAddSSHKey(ctx, req.SSHKey)
	if err != nil {
		return nil, err
	}

	order.SSHKeyID = sshKeyID
	path = fmt.Sprintf("%s%s/kronoscloud/instances", heficedAPIEndpoint, d.tenantID)

	var out heficedOrderResp
	if err := d.post(ctx, fmt.Sprintf("%s/order", path), order, &out); err != nil {
		return nil, fmt.Errorf("post failed with: %v", err)
	}

	checkTick := time.NewTicker(5 * time.Second)
	path = fmt.Sprintf("%s/%d", path, out.Items[0].InstanceID)
	for {
		select {
		case <-ctx.Done():
			return nil, ctx.Err()
		case <-checkTick.C:
			var resp heficedInstance
			if err := d.get(ctx, path, &resp); err != nil {
				return nil, err
			}
			if resp.Status == "running" {
				return heficedNode(&resp), nil
			}
		}
	}
}

// List returns a list of Node that current have a status of "running".
func (d *HeficedDriver) List(ctx context.Context, req *ListRequest) ([]*Node, error) {
	path := fmt.Sprintf("%s/%s/kronoscloud/instances?status=running", heficedAPIEndpoint, d.tenantID)
	nodes := []*Node{}

	var resp heficedInstancesResp
	if err := d.get(ctx, path, &resp); err != nil {
		return nil, err
	}

	for _, data := range resp.Data {
		for _, instance := range data {
			nodes = append(nodes, heficedNode(&instance))
		}
	}
	return nodes, nil
}

// Delete force stops the instance ID from the request, then cancels the
// instance immediately rather than at the end of the billing period.
func (d *HeficedDriver) Delete(ctx context.Context, req *DeleteRequest) error {
	path := fmt.Sprintf(
		"%s/%s/kronoscloud/instances/%s",
		heficedAPIEndpoint,
		d.tenantID,
		req.ProviderID,
	)

	if err := d.post(
		ctx,
		fmt.Sprintf("%s/stop", path),
		map[string]bool{"force": true},
		nil,
	); err != nil {
		return err
	}

	body := map[string]string{
		"type":   "Immediate",
		"reason": "ephemeral node",
	}
	return d.post(ctx, fmt.Sprintf("%s/cancel", path), body, nil)
}

func (d *HeficedDriver) findOrAddSSHKey(ctx context.Context, public string) (string, error) {
	path := fmt.Sprintf("%s/%s/sshkeys", heficedAPIEndpoint, d.tenantID)

	var keys heficedSSHKeys
	if err := d.get(ctx, path, &keys); err != nil {
		return "", err
	}

	pk, _, _, _, err := ssh.ParseAuthorizedKey([]byte(public))
	if err != nil {
		return "", err
	}

	f := ssh.FingerprintLegacyMD5(pk)
	for _, x := range keys.Data {
		if x.FingerPrint == f {
			return x.ID, nil
		}
	}

	data := map[string]string{
		"publicKey": public,
		"label":     fmt.Sprintf("infra-key-%d", time.Now().UnixNano()),
	}
	var out map[string]string
	if err := d.post(ctx, path, &data, &out); err != nil {
		return "", nil
	}

	return out["id"], nil
}

func (d *HeficedDriver) findTemplateID(ctx context.Context, path, region string) (string, error) {
	var templates heficedTemplates
	if err := d.get(
		ctx,
		fmt.Sprintf("%s/templates?instanceTypeId=linux&locationId=%s", path, region),
		&templates,
	); err != nil {
		return "", err
	}

	// TODO: check paging
	//if templates.Links.Paging.Next != "" {
	//}

	for _, template := range templates.Data {
		if template.Name == heficedOS {
			return template.ID, nil
		}
	}

	return "", fmt.Errorf("failed to find template ID for %s", heficedOS)
}

func (d *HeficedDriver) get(ctx context.Context, path string, output interface{}) error {
	resp, err := d.client.Get(path)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("request failed with: %v", resp)
	}

	body, err := ioutil.ReadAll(resp.Body)
	if err != nil {
		return err
	}

	if err := json.Unmarshal(body, output); err != nil {
		return fmt.Errorf("failed to unmarshal body: %s %v", string(body), err)
	}
	return nil
}

func (d *HeficedDriver) post(ctx context.Context, path string, input, output interface{}) error {
	marshalled, err := json.Marshal(input)
	if err != nil {
		return err
	}
	resp, err := d.client.Post(path, "application/json", bytes.NewBuffer(marshalled))
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		var body []byte
		body, err = ioutil.ReadAll(resp.Body)
		if err != nil {
			return fmt.Errorf("failed to read error body: %s %v", resp.Body, err)
		}

		var out heficedErr
		if err = json.Unmarshal(body, &out); err != nil {
			return fmt.Errorf("failed to unmarshal error body: %q %v", body, err)
		}
		return fmt.Errorf("request failed with: %+v", out)
	}

	if output == nil {
		return nil
	}

	body, err := ioutil.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("failed to read body: %s %v", resp.Body, err)
	}

	if err := json.Unmarshal(body, &output); err != nil {
		return fmt.Errorf("failed to unmarshal body: %s %v", resp.Body, err)
	}
	return nil
}

type heficedSSHKeys struct {
	Data []struct {
		ID          string `json:"id"`
		Label       string `json:"label"`
		FingerPrint string `json:"fingerPrint"`
		Created     int    `json:"created"`
	} `json:"data"`
}

type heficedTemplates struct {
	Data []struct {
		ID      string  `json:"id"`
		Created int     `json:"created"`
		Size    float64 `json:"size"`
		Version string  `json:"version"`
		Name    string  `json:"name"`
	} `json:"data"`
	Links struct {
		Paging struct {
			Self     string `json:"self"`
			Next     string `json:"next"`
			Previous string `json:"previous"`
		} `json:"paging"`
	} `json:"links"`
}

type heficedOrder struct {
	InstanceTypeID      string `json:"instanceTypeId"`
	LocationID          string `json:"locationId"`
	TemplateID          string `json:"templateId"`
	SSHKeyID            string `json:"sshKeyId"`
	Vcpu                int    `json:"vcpu"`
	Memory              int    `json:"memory"`
	Disk                int    `json:"disk"`
	BillingTypeID       int    `json:"billingTypeId"`
	AdditionalBandwidth int    `json:"additionalBandwidth"`
	UseCredit           bool   `json:"useCredit"`
}

type heficedQuote struct {
	Items []struct {
		name          string
		Total         float64 `json:"total"`
		Configuration struct {
			Raw struct {
				Product string `json:"product"`
				Vcpu    int    `json:"vcpu"`
				Memory  int    `json:"memory"`
				Disk    int    `json:"disk"`
			} `json:"raw"`
			Pricing struct {
				Raw struct {
					Vcpu   float32 `json:"vcpu"`
					Memory float32 `json:"memory"`
					Disk   float32 `json:"disk"`
				} `json:"raw"`
			} `json:"pricing"`
		} `json:"configuration"`
		Description string `json:"description"`
	} `json:"items"`
	Currency       string  `json:"currency"`
	Total          float64 `json:"total"`
	Tax            float64 `json:"tax"`
	UsageBasedRate float64 `json:"usageBasedRate"`
}

type heficedErr struct {
	Type     string `json:"type"`
	Title    string `json:"title"`
	Detail   string `json:"detail"`
	Instance string `json:"instance"`
	Context  string `json:"context"`
}

type heficedOrderResp struct {
	ID           int     `json:"id"`
	DateCreated  int     `json:"dateCreated"`
	DateDue      int     `json:"dateDue"`
	DatePaid     int     `json:"datePaid"`
	Credit       float32 `json:"credit"`
	Transactions []struct {
		ID            int     `json:"id"`
		PaymentMethod string  `json:"paymentMethod"`
		Date          int     `json:"date"`
		Description   string  `json:"description"`
		In            int     `json:"in"`
		Out           int     `json:"out"`
		Fees          float32 `json:"fees"`
		Ref           string  `json:"ref"`
		InvoiceID     int     `json:"invoiceId"`
	} `json:"transactions"`
	Items []struct {
		Total         float32     `json:"total"`
		Description   string      `json:"description"`
		InstanceID    int         `json:"instanceId"`
		PeriodStart   bool        `json:"periodStart"`
		PeriodEnd     bool        `json:"periodEnd"`
		Configuration interface{} `json:"configuration"`
	} `json:"items"`
	Currency       string  `json:"currency"`
	Total          float32 `json:"total"`
	Subtotal       float32 `json:"subtotal"`
	Tax            float32 `json:"tax"`
	Taxrate        float32 `json:"taxrate"`
	LeftToPay      float32 `json:"leftToPay"`
	Status         string  `json:"status"`
	AsyncTaskIds   []int   `json:"asyncTaskIds"`
	UsageBasedRate float32 `json:"usageBasedRate"`
}

type heficedInstancesResp struct {
	Data [][]heficedInstance `json:"data"`
}

type heficedInstance struct {
	ID           int    `json:"id"`
	Status       string `json:"status"`
	Hostname     string `json:"hostname"`
	InstanceType struct {
		ID   string `json:"id"`
		Name string `json:"name"`
	} `json:"instanceType"`
	Location struct {
		ID        string `json:"id"`
		Name      string `json:"name"`
		Continent string `json:"continent"`
	} `json:"location"`
	Template struct {
		ID   string  `json:"id"`
		Size float32 `json:"size"`
		Name string  `json:"name"`
	} `json:"template"`
	Iso     interface{} `json:"iso"`
	Network struct {
		V4 struct {
			Ipaddress     string   `json:"ipaddress"`
			Gateway       string   `json:"gateway"`
			AdditionalIps []string `json:"additionalIps"`
		} `json:"v4"`
		V6 struct {
			Ipaddress     string   `json:"ipaddress"`
			Gateway       string   `json:"gateway"`
			AdditionalIps []string `json:"additionalIps"`
		} `json:"v6"`
	} `json:"network"`
	Billing struct {
		Product string `json:"product"`
		Type    struct {
			ID   int    `json:"id"`
			Name string `json:"name"`
		} `json:"type"`
		Status             string  `json:"status"`
		HourlySpendingRate float32 `json:"hourlySpendingRate"`
		Price              float32 `json:"price"`
		StartDate          int     `json:"startDate"`
		EndDate            int     `json:"endDate"`
	} `json:"billing"`
	Vcpu            int    `json:"vcpu"`
	Memory          int    `json:"memory"`
	Disk            int    `json:"disk"`
	IP              int    `json:"ip"`
	Backup          int    `json:"backup"`
	AdditionalDisk1 int    `json:"additionalDisk1"`
	CPULimited      bool   `json:"cpuLimited"`
	NetworkLimited  bool   `json:"networkLimited"`
	Password        string `json:"password"`
}
