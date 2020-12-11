package node

import (
	"context"
	"fmt"
	"net/url"
	"strings"
	"time"

	"github.com/golang/geo/s2"
	"github.com/ovh/go-ovh/ovh"
	"golang.org/x/sync/errgroup"
)

const ovhOS = "Ubuntu 20.04"

var ovhCurrency = map[string]string{
	"CA": "CAD",
	"US": "USD",
}

// 30 days with 50% monthly billing discount
const ovhMonthlyBillingRate = 30 * 24 * 0.5

var ovhRegions = []*Region{
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
		LatLng: s2.LatLngFromDegrees(45.3151, -73.8779),
	},
	{
		Name:   "WAW1",
		City:   "Warsaw, Poland",
		LatLng: s2.LatLngFromDegrees(52.2297, 21.0122),
	},
	{
		Name:   "SYD1",
		City:   "Sydney, Australia",
		LatLng: s2.LatLngFromDegrees(-33.8688, 151.2093),
	},
	{
		Name:   "SGP1",
		City:   "Singapore",
		LatLng: s2.LatLngFromDegrees(1.3521, 103.8198),
	},
}

// NewOVHDriver ...
func NewOVHDriver(region, appKey, appSecret, consumerKey, projectID string) (*OVHDriver, error) {
	client, err := ovh.NewClient(subToFullname(region), appKey, appSecret, consumerKey)
	if err != nil {
		return nil, err
	}
	return &OVHDriver{projectID: projectID, subsidiary: region, client: client}, nil
}

func newOVHPriceMap(c *ovhCatalog) ovhPriceMap {
	m := ovhPriceMap{}
	for _, addon := range c.Addons {
		if len(addon.Pricings) == 0 {
			continue
		}
		for _, p := range addon.Pricings {
			if p.Price == 0 {
				continue
			}
			m[addon.InvoiceName] = ovhPrice{price: p.Price, tax: p.Tax}
		}
	}
	return m
}

type ovhPriceMap map[string]ovhPrice

func (m ovhPriceMap) FindByCode(code string) float64 {
	x, ok := m[strings.Split(code, ".")[0]]
	if !ok {
		// TODO: handle differently
		fmt.Printf("failed to find price code in map %q %+v\n", code, m)
		return 0
	}

	return float64(x.price) / 100000000.0
}

type ovhPrice struct {
	price int
	tax   int
}

// OVHDriver ...
type OVHDriver struct {
	projectID  string
	subsidiary string
	client     *ovh.Client
}

// Provider ...
func (d *OVHDriver) Provider() string {
	return "ovh"
}

func (d *OVHDriver) DefaultUser() string {
	return "ubuntu"
}

// Regions ...
func (d *OVHDriver) Regions(ctx context.Context, req *RegionsRequest) ([]*Region, error) {
	regions := make([]*Region, 0, len(ovhRegions))
	path := fmt.Sprintf("/cloud/project/%s/region", url.QueryEscape(d.projectID))
	resp := []string{}
	if err := d.client.GetWithContext(ctx, path, &resp); err != nil {
		return nil, err
	}

	for _, region := range resp {
		for _, x := range ovhRegions {
			if x.Name == region {
				regions = append(regions, x)
			}
		}
	}

	return regions, nil
}

// SKUs ...
func (d *OVHDriver) SKUs(ctx context.Context, req *SKUsRequest) ([]*SKU, error) {
	skus := []*SKU{}
	priceMap, err := d.loadPricesForSKUs(ctx)
	if err != nil {
		return nil, err
	}

	regions, err := d.Regions(ctx, &RegionsRequest{})
	if err != nil {
		return nil, err
	}

	if !ValidRegion(req.Region, regions) {
		return nil, fmt.Errorf("invalid region(%q)", req.Region)
	}

	names := map[string]bool{}
	path := fmt.Sprintf("/cloud/project/%s/flavor", url.QueryEscape(d.projectID))
	resp := []*ovhSKU{}
	if err := d.client.GetWithContext(
		ctx,
		fmt.Sprintf("%s?region=%s", path, url.QueryEscape(req.Region)),
		&resp,
	); err != nil {
		return nil, err
	}

	for _, s := range resp {
		if _, ok := names[s.Name]; !ok {
			names[s.Name] = true
			skus = append(skus, d.ovhSKU(s, priceMap))
		}
	}

	if len(skus) == 0 {
		return nil, fmt.Errorf("failed to find any skus")
	}

	return skus, nil
}

// Create ...
func (d *OVHDriver) Create(ctx context.Context, req *CreateRequest) (*Node, error) {
	path := fmt.Sprintf("/cloud/project/%s", url.QueryEscape(d.projectID))
	priceMap, err := d.loadPricesForSKUs(ctx)
	if err != nil {
		return nil, err
	}

	key, err := d.findOrAddKey(ctx, req.SSHKey)
	if err != nil {
		return nil, err
	}

	imageID, err := d.findImageIDForRegion(ctx, req.Region)
	if err != nil {
		return nil, err
	}

	flavorID, err := d.findFlavorIDFromName(ctx, req.SKU, req.Region)
	if err != nil {
		return nil, err
	}

	resp := ovhInstance{}
	data := map[string]string{
		"name":     req.Name,
		"region":   req.Region,
		"flavorId": flavorID,
		"imageId":  imageID,
		"sshKeyId": key,
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
				return d.ovhNode(&resp, priceMap), nil
			}
		}
	}
}

// Delete ...
func (d *OVHDriver) Delete(ctx context.Context, req *DeleteRequest) error {
	path := fmt.Sprintf("/cloud/project/%s/instance/%s", url.QueryEscape(d.projectID), url.QueryEscape(req.ProviderID))
	return d.client.DeleteWithContext(ctx, path, nil)
}

// List ...
func (d *OVHDriver) List(ctx context.Context, req *ListRequest) ([]*Node, error) {
	path := fmt.Sprintf("/cloud/project/%s/instance", url.QueryEscape(d.projectID))
	nodes := []*Node{}
	priceMap, err := d.loadPricesForSKUs(ctx)
	if err != nil {
		return nil, err
	}

	resp := []*ovhInstance{}
	if err := d.client.GetWithContext(ctx, path, &resp); err != nil {
		return nil, err
	}

	var eg errgroup.Group
	skus := map[string]*ovhSKU{}
	for _, instance := range resp {
		if _, ok := skus[instance.FlavorID]; ok {
			continue
		}
		skus[instance.FlavorID] = &ovhSKU{}
		eg.Go(func() error {
			path := fmt.Sprintf("/cloud/project/%s/flavor/%s", url.QueryEscape(d.projectID), instance.FlavorID)
			return d.client.GetWithContext(ctx, path, skus[instance.FlavorID])
		})
	}
	if err := eg.Wait(); err != nil {
		return nil, err
	}

	for _, instance := range resp {
		instance.Flavor = *skus[instance.FlavorID]
		nodes = append(nodes, d.ovhNode(instance, priceMap))
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

	var resp key
	data := map[string]string{
		"publicKey": public,
		"name":      fmt.Sprintf("infra-key-%d", time.Now().UnixNano()),
	}

	if err := d.client.PostWithContext(ctx, path, data, &resp); err != nil {
		return "", err
	}

	return resp.ID, nil
}

func (d *OVHDriver) findImageIDForRegion(ctx context.Context, region string) (string, error) {
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

func (d *OVHDriver) findFlavorIDFromName(ctx context.Context, name, region string) (string, error) {
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

func (d *OVHDriver) loadPricesForSKUs(ctx context.Context) (ovhPriceMap, error) {
	path := fmt.Sprintf("/order/catalog/public/cloud?ovhSubsidiary=%s", d.subsidiary)
	resp := &ovhCatalog{}
	if err := d.client.GetWithContext(ctx, path, resp); err != nil {
		return nil, err
	}
	return newOVHPriceMap(resp), nil
}

func (d *OVHDriver) ovhSKU(flavor *ovhSKU, priceMap ovhPriceMap) *SKU {
	return &SKU{
		Name:         flavor.Name,
		CPUs:         flavor.Vcpus,
		Memory:       flavor.RAM,
		Disk:         flavor.Disk,
		NetworkCap:   0,
		NetworkSpeed: flavor.OutboundBandwidth,
		PriceHourly: &Price{
			Value:    priceMap.FindByCode(flavor.PlanCodes.Hourly),
			Currency: ovhCurrency[d.subsidiary],
		},
		PriceMonthly: &Price{
			Value:    priceMap.FindByCode(flavor.PlanCodes.Monthly) * ovhMonthlyBillingRate,
			Currency: ovhCurrency[d.subsidiary],
		},
	}
}

func (d *OVHDriver) ovhNode(instance *ovhInstance, priceMap ovhPriceMap) *Node {
	var networks Networks
	for _, ip := range instance.IPAddresses {
		if isIPv4(ip.IP) {
			networks.V4 = append(networks.V4, ip.IP)
		} else {
			networks.V6 = append(networks.V6, ip.IP)
		}
	}

	var region *Region
	for _, r := range ovhRegions {
		if instance.Region == r.Name {
			region = r
		}
	}

	return &Node{
		ProviderID: instance.ID,
		Name:       instance.Name,
		Memory:     instance.Flavor.RAM,
		CPUs:       instance.Flavor.Vcpus,
		Disk:       instance.Flavor.Disk,
		Networks:   &networks,
		Status:     instance.Status,
		SKU:        d.ovhSKU(&instance.Flavor, priceMap),
		Region:     region,
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
	Status   string    `json:"status"`
	Created  time.Time `json:"created"`
	Region   string    `json:"region"`
	Flavor   ovhSKU    `json:"flavor"`
	FlavorID string    `json:"flavorId"`
	Image    struct {
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

type ovhCatalog struct {
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
