// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package node

import (
	"context"
	"fmt"
	"strings"
	"time"

	compute "cloud.google.com/go/compute/apiv1"
	"cloud.google.com/go/compute/apiv1/computepb"
	"github.com/golang/geo/s2"
	"github.com/googleapis/gax-go/v2"
	"golang.org/x/oauth2/google"
	"google.golang.org/api/iterator"
	"google.golang.org/api/option"
	"google.golang.org/grpc/codes"
	"google.golang.org/protobuf/proto"
)

var gcpRegions = []*Region{
	{
		Name: "asia-east1",
		City: "Changhua County, Taiwan",
	},
	{
		Name: "asia-east1",
		City: "Hong Kong, China",
	},
	{
		Name: "asia-northeast1",
		City: "Tokyo, Japan",
	},
	{
		Name: "asia-northeast2",
		City: "Osaka, Japan",
	},
	{
		Name: "asia-northeast3",
		City: "Seoul, South Korea",
	},
	{
		Name: "asia-south1",
		City: "Mumbai, India",
	},
	{
		Name: "asia-south2",
		City: "Delhi, India",
	},
}

const gcpOS = "ubuntu-2204-lts"

var _ Driver = &GCPDriver{}

type GCPDriver struct {
	creds *google.Credentials
}

func NewGCPDriver(credentials []byte) (*GCPDriver, error) {
	creds, err := google.CredentialsFromJSON(context.Background(), credentials, compute.DefaultAuthScopes()...)
	if err != nil {
		return nil, err
	}
	return &GCPDriver{creds}, nil
}

func (d *GCPDriver) Provider() string {
	return "gcp"
}

// TODO: add long/lat to this
func (d *GCPDriver) Regions(ctx context.Context, req *RegionsRequest) ([]*Region, error) {
	client, err := compute.NewRegionsRESTClient(ctx, option.WithCredentials(d.creds))
	if err != nil {
		return nil, fmt.Errorf("NewRegionsRESTClient: %v", err)
	}
	defer client.Close()

	var regions []*Region
	regionsListRequest := &computepb.ListRegionsRequest{Project: d.creds.ProjectID}
	iter := client.List(ctx, regionsListRequest)
	for {
		region, err := iter.Next()
		if err == iterator.Done {
			break
		}
		if err != nil {
			return nil, fmt.Errorf("unable to iterate: %w", err)
		}
		for _, zone := range region.GetZones() {
			zoneAsArr := strings.Split(zone, "/")
			regions = append(regions, &Region{
				Name: zoneAsArr[len(zoneAsArr)-1],
			})
		}
	}

	return regions, nil
}

func (d *GCPDriver) SKUs(ctx context.Context, req *SKUsRequest) ([]*SKU, error) {
	client, err := compute.NewMachineTypesRESTClient(ctx, option.WithCredentials(d.creds))
	if err != nil {
		return nil, fmt.Errorf("NewMachineTypesRESTClient: %v", err)
	}
	defer client.Close()

	var skus []*SKU
	listMachineTypesListRequest := &computepb.ListMachineTypesRequest{
		Project: d.creds.ProjectID,
		Zone:    req.Region,
	}
	iter := client.List(ctx, listMachineTypesListRequest)
	for {
		resp, err := iter.Next()
		if err == iterator.Done {
			break
		}
		if err != nil {
			return nil, err
		}
		skus = append(skus, &SKU{
			Name: resp.GetName(),
		})
	}
	return skus, nil
}

func (d *GCPDriver) Create(ctx context.Context, req *CreateRequest) (*Node, error) {
	computeClient, err := compute.NewInstancesRESTClient(ctx, option.WithCredentials(d.creds))
	if err != nil {
		return nil, fmt.Errorf("NewInstancesRESTClient: %v", err)
	}
	defer computeClient.Close()

	imagesClient, err := compute.NewImagesRESTClient(ctx, option.WithCredentials(d.creds))
	if err != nil {
		return nil, fmt.Errorf("NewImagesRESTClient: %v", err)
	}
	defer imagesClient.Close()

	// List of public operating system (OS) images: https://cloud.google.com/compute/docs/images/os-details.
	getImageFamilyRequest := &computepb.GetFromFamilyImageRequest{
		Project: "ubuntu-os-cloud",
		Family:  gcpOS,
	}
	newestUbuntu, err := imagesClient.GetFromFamily(ctx, getImageFamilyRequest)
	if err != nil {
		return nil, fmt.Errorf("unable to get image from family: %v", err)
	}

	insertInstanceRequest := &computepb.InsertInstanceRequest{
		Project: d.creds.ProjectID,
		Zone:    req.Region,
		InstanceResource: &computepb.Instance{
			Name: proto.String(req.Name),
			Metadata: &computepb.Metadata{
				Items: []*computepb.Items{
					{
						Key:   proto.String("ssh-keys"),
						Value: proto.String(fmt.Sprintf("%s:%s", d.DefaultUser(), req.SSHKey)),
					},
				},
			},
			Disks: []*computepb.AttachedDisk{
				{
					InitializeParams: &computepb.AttachedDiskInitializeParams{
						DiskSizeGb:  proto.Int64(30),
						SourceImage: newestUbuntu.SelfLink,
						DiskType:    proto.String(fmt.Sprintf("zones/%s/diskTypes/pd-standard", req.Region)),
					},
					AutoDelete: proto.Bool(true),
					Boot:       proto.Bool(true),
					Type:       proto.String(computepb.AttachedDisk_PERSISTENT.String()),
				},
			},
			MachineType: proto.String(fmt.Sprintf("zones/%s/machineTypes/%s", req.Region, req.SKU)),
			NetworkInterfaces: []*computepb.NetworkInterface{
				{
					Name: proto.String("global/networks/default"),
					AccessConfigs: []*computepb.AccessConfig{
						{
							Name:        proto.String("External NAT"),
							NetworkTier: proto.String("Premium"), // Standard // for more performant internetwork communication, use "Premium"
						},
					},
				},
			},
		},
	}

	if req.Spot {
		insertInstanceRequest.InstanceResource.Scheduling = &computepb.Scheduling{
			ProvisioningModel:         proto.String("SPOT"),
			InstanceTerminationAction: proto.String("DELETE"),
		}
	}

	op, err := computeClient.Insert(ctx, insertInstanceRequest)
	if err != nil {
		return nil, fmt.Errorf("unable to create instance: %v", err)
	}

	if err = op.Wait(ctx); err != nil {
		return nil, fmt.Errorf("unable to wait for the operation: %v", err)
	}

	withRetry := gax.WithRetry(func() gax.Retryer {
		return gax.OnCodes([]codes.Code{
			codes.Unavailable,
			codes.NotFound,
		}, gax.Backoff{
			Initial:    time.Second,
			Max:        10 * time.Second,
			Multiplier: 2,
		})
	})

	getInstanceRequest := &computepb.GetInstanceRequest{
		Instance: req.Name,
		Project:  d.creds.ProjectID,
		Zone:     req.Region,
	}
	instance, err := computeClient.Get(ctx, getInstanceRequest, withRetry)
	if err != nil {
		return nil, fmt.Errorf("failed getting instance: %w", err)
	}

	// TODO: do this
	return &Node{
		User:         d.DefaultUser(),
		ProviderName: d.Provider(),
		ProviderID:   fmt.Sprint(instance.GetId()),
		Name:         instance.GetName(),
		Memory:       0,
		CPUs:         0,
		Disk:         int(instance.GetDisks()[0].GetDiskSizeGb()),
		Networks: &Networks{
			V4: []string{instance.GetNetworkInterfaces()[0].GetAccessConfigs()[0].GetNatIP()},
		},
		Status: true,
		SKU: &SKU{
			Name:         req.SKU,
			PriceMonthly: &Price{Value: 0},
			PriceHourly:  &Price{Value: 0},
		},
		Region: &Region{
			Name:   req.Region,
			LatLng: s2.LatLngFromDegrees(0, 0),
		},
	}, nil
}

func (d *GCPDriver) List(ctx context.Context, req *ListRequest) ([]*Node, error) {
	client, err := compute.NewInstancesRESTClient(ctx, option.WithCredentials(d.creds))
	if err != nil {
		return nil, fmt.Errorf("NewInstancesRESTClient: %v", err)
	}
	defer client.Close()

	var nodes []*Node
	listInstancesRequest := &computepb.ListInstancesRequest{Project: d.creds.ProjectID}
	iter := client.List(ctx, listInstancesRequest)
	for {
		resp, err := iter.Next()
		if err == iterator.Done {
			break
		}
		if err != nil {
			return nil, err
		}
		nodes = append(nodes, &Node{
			Name: resp.GetName(),
		})
	}
	return nodes, nil
}

func (d *GCPDriver) Delete(ctx context.Context, req *DeleteRequest) error {
	client, err := compute.NewInstancesRESTClient(ctx, option.WithCredentials(d.creds))
	if err != nil {
		return fmt.Errorf("NewInstancesRESTClient: %v", err)
	}
	defer client.Close()

	deleteInstanceRequest := &computepb.DeleteInstanceRequest{
		Instance: req.ProviderID,
		Zone:     req.Region,
		Project:  d.creds.ProjectID,
	}
	op, err := client.Delete(ctx, deleteInstanceRequest)
	if err != nil {
		return fmt.Errorf("failed to delete instance: %v", err)
	}

	if err = op.Wait(ctx); err != nil {
		return fmt.Errorf("unable to wait for the operation: %v", err)
	}

	return nil
}

func (d *GCPDriver) DefaultUser() string {
	// TODO: get this from config or something
	return "strimsgg"
}
