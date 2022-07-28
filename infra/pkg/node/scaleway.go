// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package node

import (
	"context"
	"fmt"
	"time"

	"github.com/golang/geo/s2"
	account "github.com/scaleway/scaleway-sdk-go/api/account/v2alpha1"
	"github.com/scaleway/scaleway-sdk-go/api/instance/v1"
	"github.com/scaleway/scaleway-sdk-go/scw"
	"golang.org/x/sync/errgroup"
)

const scalewayOS = "Ubuntu 20.04 Focal Fossa"

var scalewayRegions = []*Region{
	{
		Name:   "fr-par-1",
		City:   "Paris, France",
		LatLng: s2.LatLngFromDegrees(48.8667, 2.3333),
	},
	{
		Name:   "nl-ams-1",
		City:   "Amsterdam, The Netherlands",
		LatLng: s2.LatLngFromDegrees(52.3500, 4.9166),
	},
}

// NewScalewayDriver ...
func NewScalewayDriver(organizationID, accessKey, secretKey string) (*ScalewayDriver, error) {
	client, err := scw.NewClient(
		scw.WithDefaultOrganizationID(organizationID),
		scw.WithAuth(accessKey, secretKey),
	)
	if err != nil {
		return nil, err
	}
	return &ScalewayDriver{client: client}, nil
}

// ScalewayDriver ...
type ScalewayDriver struct {
	client *scw.Client
}

// Provider ...
func (d *ScalewayDriver) Provider() string {
	return "scaleway"
}

func (d *ScalewayDriver) DefaultUser() string {
	return "root"
}

// Regions ...
func (d *ScalewayDriver) Regions(ctx context.Context, req *RegionsRequest) ([]*Region, error) {
	return append(make([]*Region, 0, len(scalewayRegions)), scalewayRegions...), nil
}

// SKUs ...
func (d *ScalewayDriver) SKUs(ctx context.Context, req *SKUsRequest) ([]*SKU, error) {
	skus := []*SKU{}

	names := map[string]bool{}
	for _, region := range scalewayRegions {
		if req.Region != "" && req.Region != region.Name {
			continue
		}

		opt := &instance.ListServersTypesRequest{
			Zone:    scw.Zone(region.Name),
			PerPage: scw.Uint32Ptr(100),
		}
		var page int32 = 0
		for {
			res, err := instance.NewAPI(d.client).ListServersTypes(opt, scw.WithContext(ctx))
			if err != nil {
				return nil, err
			}

			for name, server := range res.Servers {
				if _, ok := names[name]; !ok {
					names[name] = true
					skus = append(skus, scalewaySKU(name, server))
				}
			}

			page++
			if len(res.Servers) == 0 || res.TotalCount >= *opt.PerPage*uint32(page) {
				break
			}
			opt.Page = scw.Int32Ptr(page)
		}
	}

	return skus, nil
}

func scalewaySKU(name string, serverType *instance.ServerType) *SKU {
	return &SKU{
		Name:         name,
		CPUs:         int(serverType.Ncpus),
		Memory:       int(serverType.RAM / (1 << 20)),
		Disk:         int(serverType.VolumesConstraint.MinSize / scw.GB),
		NetworkCap:   0,
		NetworkSpeed: int(*serverType.Network.SumInternetBandwidth / (1 << 20)),
		PriceHourly: &Price{
			Value:    float64(serverType.HourlyPrice),
			Currency: "EUR",
		},
		PriceMonthly: &Price{
			Value:    float64(serverType.MonthlyPrice),
			Currency: "EUR",
		},
	}
}

func (d *ScalewayDriver) findOrAddKey(ctx context.Context, public string) (*account.SSHKey, error) {
	opt := &account.ListSSHKeysRequest{
		PageSize: scw.Uint32Ptr(100),
	}
	var page int32 = 0
	for {
		res, err := account.NewAPI(d.client).ListSSHKeys(opt, scw.WithContext(ctx))
		if err != nil {
			return nil, err
		}

		for _, key := range res.SSHKeys {
			if key.PublicKey == public {
				return key, nil
			}
		}

		page++
		if len(res.SSHKeys) == 0 || res.TotalCount >= *opt.PageSize*uint32(page) {
			break
		}
		opt.Page = scw.Int32Ptr(page)
	}

	key, err := account.NewAPI(d.client).CreateSSHKey(&account.CreateSSHKeyRequest{
		Name:      fmt.Sprintf("key-%d", time.Now().UnixNano()),
		PublicKey: public,
	}, scw.WithContext(ctx))
	return key, err
}

func (d *ScalewayDriver) findLatestImage(ctx context.Context, region, sku string) (*instance.Image, error) {
	serverType, err := instance.NewAPI(d.client).GetServerType(&instance.GetServerTypeRequest{
		Zone: scw.Zone(region),
		Name: sku,
	})
	if err != nil {
		return nil, err
	}

	images, err := instance.NewAPI(d.client).ListImages(&instance.ListImagesRequest{
		Zone:    scw.Zone(region),
		Name:    scw.StringPtr(scalewayOS),
		Arch:    scw.StringPtr(serverType.Arch.String()),
		PerPage: scw.Uint32Ptr(100),
	}, scw.WithContext(ctx))
	if err != nil {
		return nil, err
	}
	if len(images.Images) == 0 {
		return nil, fmt.Errorf("image not found for OS: %s", scalewayOS)
	}

	latestImage := images.Images[0]
	for _, image := range images.Images {
		if image.CreationDate.After(*latestImage.CreationDate) {
			latestImage = image
		}
	}
	return latestImage, nil
}

// Create ...
func (d *ScalewayDriver) Create(ctx context.Context, req *CreateRequest) (*Node, error) {
	image, err := d.findLatestImage(ctx, req.Region, req.SKU)
	if err != nil {
		return nil, err
	}

	if _, err = d.findOrAddKey(ctx, req.SSHKey); err != nil {
		return nil, err
	}

	createRes, err := instance.NewAPI(d.client).CreateServer(&instance.CreateServerRequest{
		Zone:           scw.Zone(req.Region),
		Name:           req.Name,
		CommercialType: req.SKU,
		Image:          image.ID,
		EnableIPv6:     true,
	}, scw.WithContext(ctx))
	if err != nil {
		return nil, err
	}

	err = instance.NewAPI(d.client).ServerActionAndWait(&instance.ServerActionAndWaitRequest{
		Zone:     scw.Zone(req.Region),
		ServerID: createRes.Server.ID,
		Action:   instance.ServerActionPoweron,
	})
	if err != nil {
		return nil, err
	}

	getRes, err := instance.NewAPI(d.client).GetServer(&instance.GetServerRequest{
		Zone:     scw.Zone(req.Region),
		ServerID: createRes.Server.ID,
	}, scw.WithContext(ctx))
	if err != nil {
		return nil, err
	}
	return d.scalewayNode(ctx, getRes.Server)
}

// List ...
func (d *ScalewayDriver) List(ctx context.Context, req *ListRequest) ([]*Node, error) {
	nodes := []*Node{}

	for _, region := range scalewayRegions {
		opt := &instance.ListServersRequest{
			Zone:    scw.Zone(region.Name),
			PerPage: scw.Uint32Ptr(100),
		}
		var page int32 = 0
		for {
			res, err := instance.NewAPI(d.client).ListServers(opt, scw.WithContext(ctx))
			if err != nil {
				return nil, err
			}

			for _, server := range res.Servers {
				node, err := d.scalewayNode(ctx, server)
				if err != nil {
					return nil, err
				}
				nodes = append(nodes, node)
			}

			page++
			if len(res.Servers) == 0 || res.TotalCount >= *opt.PerPage*uint32(page) {
				break
			}
			opt.Page = scw.Int32Ptr(page)
		}
	}

	return nodes, nil
}

// Delete ...
func (d *ScalewayDriver) Delete(ctx context.Context, req *DeleteRequest) error {
	getRes, err := instance.NewAPI(d.client).GetServer(&instance.GetServerRequest{
		Zone:     scw.Zone(req.Region),
		ServerID: req.ProviderID,
	}, scw.WithContext(ctx))
	if err != nil {
		return err
	}

	err = instance.NewAPI(d.client).ServerActionAndWait(&instance.ServerActionAndWaitRequest{
		Zone:     scw.Zone(req.Region),
		ServerID: req.ProviderID,
		Action:   instance.ServerActionPoweroff,
	})
	if err != nil {
		return err
	}

	err = instance.NewAPI(d.client).DeleteServer(&instance.DeleteServerRequest{
		Zone:     scw.Zone(req.Region),
		ServerID: req.ProviderID,
	}, scw.WithContext(ctx))
	if err != nil {
		return err
	}

	var eg errgroup.Group
	for _, volume := range getRes.Server.Volumes {
		volume := volume
		eg.Go(func() error {
			return instance.NewAPI(d.client).DeleteVolume(&instance.DeleteVolumeRequest{
				Zone:     scw.Zone(req.Region),
				VolumeID: volume.ID,
			}, scw.WithContext(ctx))
		})
	}
	return eg.Wait()
}

func (d *ScalewayDriver) scalewayNode(ctx context.Context, server *instance.Server) (*Node, error) {
	opt := &instance.GetServerTypeRequest{
		Zone: scw.Zone(server.Zone),
		Name: server.CommercialType,
	}
	serverType, err := instance.NewAPI(d.client).GetServerType(opt)
	if err != nil {
		return nil, err
	}

	node := &Node{
		ProviderID: server.ID,
		Name:       server.Name,
		CPUs:       int(serverType.Ncpus),
		Memory:     int(serverType.RAM / (1 << 20)),
		Disk:       int(serverType.VolumesConstraint.MinSize / scw.GB),
		Status:     server.State == instance.ServerStateStarting || server.State == instance.ServerStateRunning,
		SKU:        scalewaySKU(server.CommercialType, serverType),
		Networks: &Networks{
			V4: []string{server.PublicIP.Address.String()},
			V6: []string{server.IPv6.Address.String()},
		},
	}

	for _, region := range scalewayRegions {
		if region.Name == server.Zone.String() {
			node.Region = region
		}
	}

	return node, nil
}
