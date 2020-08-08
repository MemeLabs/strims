package node

import (
	"context"
	"fmt"
	"net/url"
	"strings"
	"time"

	"github.com/golang/geo/s2"
	"github.com/ovh/go-ovh/ovh"
)

const ovhOS = "Ubuntu 20.04"

var ovhCurrency = map[string]string{
	"CA": "CAD",
	"US": "USD",
}

var ovhRegions = []*Region{
	/*
		{
			Name:   "VIN1",
			City:   "Virginia, United States",
			LatLng: s2.LatLngFromDegrees(38.7465, 77.6738),
		},
		{
			Name:   "HIL1",
			City:   "Oregon, United States",
			LatLng: s2.LatLngFromDegrees(45.5272, 122.9361),
		},
	*/
	{
		Name:   "UK1",
		City:   "London, United Kingdom",
		LatLng: s2.LatLngFromDegrees(51.5074, 0.1278),
	},
	{
		Name:   "GRA7",
		City:   "Gravelines, France",
		LatLng: s2.LatLngFromDegrees(50.9871, 2.1255),
	},
	{
		Name:   "SBG5",
		City:   "Strasbourg, France",
		LatLng: s2.LatLngFromDegrees(48.5734, 7.7521),
	},
	{
		Name:   "DE1",
		City:   "Frankfurt, Germany",
		LatLng: s2.LatLngFromDegrees(50.1109, 8.6821),
	},
	{
		Name:   "BHS5",
		City:   "Beauharnois, Quebec, Canada",
		LatLng: s2.LatLngFromDegrees(45.3151, 73.8779),
	},
	{
		Name:   "WAW1",
		City:   "Warsaw, Poland",
		LatLng: s2.LatLngFromDegrees(52.2297, 21.0122),
	},
	{
		Name:   "SYD1",
		City:   "Sydney, Australia",
		LatLng: s2.LatLngFromDegrees(33.8688, 151.2093),
	},
	{
		Name:   "SGP1",
		City:   "Singapore",
		LatLng: s2.LatLngFromDegrees(1.3521, 103.8198),
	},
}

func NewOVHDriver(region, appKey, appSecret, consumerKey, projectID string) (*OVHDriver, error) {
	client, err := ovh.NewClient(subToFullname(region), appKey, appSecret, consumerKey)
	if err != nil {
		return nil, err
	}
	return &OVHDriver{projectID: projectID, subsidiary: region, client: client}, nil
}

type price struct {
	price int
	tax   int
}

type OVHDriver struct {
	projectID  string
	subsidiary string
	pricemap   map[string]price
	client     *ovh.Client
}

func (d *OVHDriver) Provider() string {
	return "ovh"
}

func (d *OVHDriver) Regions(ctx context.Context, req *RegionsRequest) ([]*Region, error) {
	return append(make([]*Region, 0, len(ovhRegions)), ovhRegions...), nil
}

func (d *OVHDriver) SKUs(ctx context.Context, req *SKUsRequest) ([]*SKU, error) {
	skus := []*SKU{}
	pricemap, err := d.loadPricesForSKUs(ctx)
	if err != nil {
		return nil, err
	}
	d.pricemap = pricemap

	path := fmt.Sprintf("/cloud/project/%s/flavor", d.projectID)
	for _, region := range ovhRegions {
		if req.Region != "" && req.Region != region.Name {
			continue
		}
		resp := []*ovhSKU{}
		if err := d.client.GetWithContext(
			ctx,
			fmt.Sprintf("%s?region=%s", path, url.QueryEscape(region.Name)),
			&resp,
		); err != nil {
			return nil, err
		}

		for _, s := range resp {
			skus = append(skus, d.ovhSKU(s))
		}
	}

	return skus, nil
}

func (d *OVHDriver) Create(ctx context.Context, req *CreateRequest) (*Node, error) {
	path := fmt.Sprintf("/cloud/project/%s", url.QueryEscape(d.projectID))
	pricemap, err := d.loadPricesForSKUs(ctx)
	if err != nil {
		return nil, err
	}
	d.pricemap = pricemap

	sshkeyIDs := []string{}
	for _, public := range req.SSHKeys {
		key, err := d.findOrAddKey(ctx, public)
		if err != nil {
			return nil, err
		}
		sshkeyIDs = append(sshkeyIDs, key)
	}

	imageID, err := d.findImageIdForRegion(ctx, req.Region)
	if err != nil {
		return nil, err
	}

	flavorID, err := d.findFlavorIdFromName(ctx, req.SKU, req.Region)
	if err != nil {
		return nil, err
	}

	resp := ovhInstance{}
	data := map[string]string{
		"name":     req.Name,
		"region":   req.Region,
		"flavorId": flavorID,
		"imageId":  imageID,
		"sshKeyId": sshkeyIDs[0], // TODO: handle multiple ssh keys or decide against it
	}

	if err := d.client.PostWithContext(ctx, fmt.Sprintf("%s/instance", path), data, &resp); err != nil {
		return nil, err
	}

	path = fmt.Sprintf("%s/instance/%s", path, url.QueryEscape(resp.ID))
	checkTick := time.NewTicker(5 * time.Second)
	for {
		select {
		case <-ctx.Done():
			return nil, ctx.Err()
		case <-checkTick.C:
			if err := d.client.GetWithContext(ctx, path, &resp); err != nil {
				return nil, err
			}

			if resp.Status == "ACTIVE" {
				return d.ovhNode(&resp), nil
			}
		}
	}
}

func (d *OVHDriver) Delete(ctx context.Context, req *DeleteRequest) error {
	path := fmt.Sprintf("/cloud/project/%s/instance/%s", url.QueryEscape(d.projectID), url.QueryEscape(req.ProviderID))
	return d.client.DeleteWithContext(ctx, path, nil)
}

func (d *OVHDriver) List(ctx context.Context, req *ListRequest) ([]*Node, error) {
	path := fmt.Sprintf("/cloud/project/%s/instance", url.QueryEscape(d.projectID))
	nodes := []*Node{}
	pricemap, err := d.loadPricesForSKUs(ctx)
	if err != nil {
		return nil, err
	}
	d.pricemap = pricemap

	resp := []*ovhInstance{}
	if err := d.client.GetWithContext(ctx, path, &resp); err != nil {
		return nil, err
	}

	for _, instance := range resp {
		nodes = append(nodes, d.ovhNode(instance))
	}

	return nodes, nil
}

func (d *OVHDriver) findOrAddKey(ctx context.Context, public string) (string, error) {
	path := fmt.Sprintf("/cloud/project/%s/sshkey", url.QueryEscape(d.projectID))
	type key struct {
		ID        string `json:"id"`
		PublicKey string `json:"publicKey"`
	}

	keys := []key{}
	if err := d.client.GetWithContext(ctx, path, &keys); err != nil {
		return "", err
	}

	for _, key := range keys {
		if key.PublicKey == public {
			return key.ID, nil
		}
	}

	var resp *key
	data := map[string]string{
		"publicKey": public,
		"name":      fmt.Sprintf("infra-key-%d", time.Now().UnixNano()),
	}

	if err := d.client.PostWithContext(ctx, path, data, &resp); err != nil {
		return "", err
	}

	return resp.ID, nil
}

func (d *OVHDriver) findImageIdForRegion(ctx context.Context, region string) (string, error) {
	path := fmt.Sprintf("/cloud/project/%s/image?osType=linux&region=%s", url.QueryEscape(d.projectID), region)
	images := []struct {
		ID   string `json:"id"`
		Name string `json:"name"`
	}{}
	if err := d.client.GetWithContext(ctx, path, &images); err != nil {
		return "", err
	}

	for _, image := range images {
		if image.Name == ovhOS {
			return image.ID, nil
		}
	}

	return "", fmt.Errorf("failed to find %s in %s", ovhOS, region)
}

func (d *OVHDriver) findFlavorIdFromName(ctx context.Context, name, region string) (string, error) {
	path := fmt.Sprintf("/cloud/project/%s/flavor?region=%s", url.QueryEscape(d.projectID), region)
	flavors := []struct {
		ID   string `json:"id"`
		Name string `json:"name"`
	}{}
	if err := d.client.GetWithContext(ctx, path, &flavors); err != nil {
		return "", err
	}

	for _, flavor := range flavors {
		if flavor.Name == strings.ToLower(name) {
			return flavor.ID, nil
		}
	}
	return "", fmt.Errorf("failed to find %s", name)
}

func (d *OVHDriver) loadPricesForSKUs(ctx context.Context) (map[string]price, error) {
	path := fmt.Sprintf("/order/catalog/public/cloud?ovhSubsidiary=%s", d.subsidiary)
	resp := catalog{}
	if err := d.client.GetWithContext(ctx, path, &resp); err != nil {
		return nil, err
	}

	out := make(map[string]price)
	for _, addon := range resp.Addons {
		if len(addon.Pricings) == 0 {
			continue
		}
		for _, p := range addon.Pricings {
			if p.Price == 0 {
				continue
			}
			out[addon.InvoiceName] = price{price: p.Price, tax: p.Tax}
		}
	}

	return out, nil
}

func priceForPlan(pricemap map[string]price, code string) float64 {
	x, ok := pricemap[strings.Split(code, ".")[0]]
	if !ok {
		// TODO: handle differently
		fmt.Printf("failed to find price code in map %q %+v\n", code, pricemap)
		return 0
	}

	return float64(x.price) / float64(100000000)
}

func (d *OVHDriver) ovhSKU(flavor *ovhSKU) *SKU {
	return &SKU{
		Name:         flavor.Name,
		CPUs:         flavor.Vcpus,
		Memory:       flavor.RAM,
		NetworkCap:   0,
		NetworkSpeed: flavor.OutboundBandwidth,
		PriceHourly:  &Price{priceForPlan(d.pricemap, flavor.PlanCodes.Hourly), ovhCurrency[d.subsidiary]},
		PriceMonthly: &Price{priceForPlan(d.pricemap, flavor.PlanCodes.Monthly), ovhCurrency[d.subsidiary]},
	}
}

func (d *OVHDriver) ovhNode(instance *ovhInstance) *Node {
	v4s, v6s := []string{}, []string{}
	for _, ip := range instance.IPAddresses {
		if isIPv4(ip.IP) {
			v4s = append(v4s, ip.IP)
		} else {
			v6s = append(v6s, ip.IP)
		}
	}
	return &Node{
		ProviderID: instance.ID,
		Name:       instance.Name,
		Memory:     instance.Flavor.RAM,
		CPUs:       instance.Flavor.Vcpus,
		Disk:       instance.Flavor.Disk,
		Networks:   &Networks{V4: v4s, V6: v6s},
		Status:     instance.Status,
		SKU:        d.ovhSKU(&instance.Flavor),
	}
}

func subToFullname(sub string) string {
	switch sub {
	case "CA":
		return "ovh-ca"
	default:
		return ""
	}
}

func isIPv4(address string) bool {
	return strings.Count(address, ":") < 2
}

type ovhSKU struct {
	ID                string `json:"id"`
	Name              string `json:"name"`
	Region            string `json:"region"`
	RAM               int    `json:"ram"`
	Disk              int    `json:"disk"`
	Vcpus             int    `json:"vcpus"`
	Type              string `json:"type"`
	OsType            string `json:"osType"`
	InboundBandwidth  int    `json:"inboundBandwidth"`
	OutboundBandwidth int    `json:"outboundBandwidth"`
	Available         bool   `json:"available"`
	PlanCodes         struct {
		Monthly string `json:"monthly"`
		Hourly  string `json:"hourly"`
	} `json:"planCodes"`
	Capabilities []struct {
		Name    string `json:"name"`
		Enabled bool   `json:"enabled"`
	} `json:"capabilities"`
	Quota int `json:"quota"`
}

type ovhInstance struct {
	ID          string `json:"id"`
	Name        string `json:"name"`
	IPAddresses []struct {
		IP        string `json:"ip"`
		Type      string `json:"type"`
		Version   int    `json:"version"`
		NetworkID string `json:"networkId"`
		GatewayIP string `json:"gatewayIp"`
	} `json:"ipAddresses"`
	Status  string    `json:"status"`
	Created time.Time `json:"created"`
	Region  string    `json:"region"`
	Flavor  ovhSKU    `json:"flavor"`
	Image   struct {
		ID           string        `json:"id"`
		Name         string        `json:"name"`
		Region       string        `json:"region"`
		Visibility   string        `json:"visibility"`
		Type         string        `json:"type"`
		MinDisk      int           `json:"minDisk"`
		MinRAM       int           `json:"minRam"`
		Size         float64       `json:"size"`
		CreationDate time.Time     `json:"creationDate"`
		Status       string        `json:"status"`
		User         string        `json:"user"`
		FlavorType   interface{}   `json:"flavorType"`
		Tags         []interface{} `json:"tags"`
		PlanCode     interface{}   `json:"planCode"`
	} `json:"image"`
	SSHKey         interface{}   `json:"sshKey"`
	MonthlyBilling interface{}   `json:"monthlyBilling"`
	PlanCode       string        `json:"planCode"`
	OperationIds   []interface{} `json:"operationIds"`
}

type catalog struct {
	Addons []struct {
		PlanCode       string        `json:"planCode"`
		InvoiceName    string        `json:"invoiceName"`
		Blobs          interface{}   `json:"blobs"`
		Family         interface{}   `json:"family"`
		Product        string        `json:"product"`
		PricingType    string        `json:"pricingType"`
		Configurations []interface{} `json:"configurations"`
		AddonFamilies  []interface{} `json:"addonFamilies"`
		Pricings       []struct {
			Phase           int           `json:"phase"`
			MustBeCompleted bool          `json:"mustBeCompleted"`
			Capacities      []string      `json:"capacities"`
			Interval        int           `json:"interval"`
			Tax             int           `json:"tax"`
			Mode            string        `json:"mode"`
			Price           int           `json:"price"`
			Promotions      []interface{} `json:"promotions"`
			Description     string        `json:"description"`
			Repeat          struct {
				Max int `json:"max"`
				Min int `json:"min"`
			} `json:"repeat"`
			Commitment int    `json:"commitment"`
			Strategy   string `json:"strategy"`
			Type       string `json:"type"`
			Quantity   struct {
				Min int         `json:"min"`
				Max interface{} `json:"max"`
			} `json:"quantity"`
			IntervalUnit string `json:"intervalUnit"`
		} `json:"pricings"`
	} `json:"addons"`
}
