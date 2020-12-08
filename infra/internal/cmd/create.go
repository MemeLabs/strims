package cmd

import (
	"crypto/rand"
	"errors"
	"fmt"

	be "github.com/MemeLabs/go-ppspp/infra/internal/backend"
	"github.com/MemeLabs/go-ppspp/infra/pkg/node"
	"github.com/spf13/cobra"
)

func init() {
	rootCmd.AddCommand(createCmd)
}

var createCmd = &cobra.Command{
	Use:   "create [provider] [sku] [region] | create custom [user] [hostname] [ipv4]",
	Short: "Create node",
	Args: func(cmd *cobra.Command, args []string) error {
		provider := args[0]
		if provider == "custom" {
			user := args[1]
			hostname := args[2]
			ipv4 := args[3]
			if user == "" || hostname == "" || ipv4 == "" {
				return errors.New("user and ipv4 are required for custom provisioning")
			}

			if !be.IsPublicIP(ipv4) {
				return fmt.Errorf("ipv4(%s) provided is not a public ip", ipv4)
			}
		} else {
			d, ok := backend.NodeDrivers[provider]
			if !ok {
				return fmt.Errorf("unsupported provider: %s", provider)
			}

			regions, err := d.Regions(cmd.Context(), &node.RegionsRequest{})
			if err != nil {
				return fmt.Errorf("failed to get regions for current driver: %w", err)
			}

			region := args[2]
			if !node.ValidRegion(region, regions) {
				return fmt.Errorf("invalid region for %q", provider)
			}

			skus, err := d.SKUs(cmd.Context(), &node.SKUsRequest{Region: region})
			if err != nil {
				return fmt.Errorf("failed to get skus for %q", provider)
			}

			sku := args[1]
			if !node.ValidSKU(sku, skus) {
				return fmt.Errorf("invalid sku for %q", provider)
			}
		}

		return nil
	},
	ValidArgsFunction: providerValidArgsFunc,
	RunE: func(cmd *cobra.Command, args []string) error {
		provider := args[0]
		if provider == "custom" {
			user := args[1]
			hostname := args[2]
			ipv4 := args[3]

			if err := backend.CreateNode(
				cmd.Context(),
				nil, // no driver
				hostname,
				"", // no region
				"", // no sku
				user,
				ipv4,
				node.Custom,
			); err != nil {
				return fmt.Errorf("failed to create node: %w", err)
			}
		} else {
			sku := args[1]
			region := args[2]

			d, ok := backend.NodeDrivers[provider]
			if !ok {
				return fmt.Errorf("invalid node provider for %q", provider)
			}

			if err := backend.CreateNode(
				cmd.Context(),
				d,
				generateHostname(provider, region),
				region,
				sku,
				d.DefaultUser(),
				"", // no ipv4
				node.Hourly,
			); err != nil {
				return fmt.Errorf("failed to create node: %w", err)
			}
		}

		return nil
	},
}

func generateHostname(provider, region string) string {
	name := make([]byte, 4)
	if _, err := rand.Read(name); err != nil {
		panic(err)
	}
	return fmt.Sprintf("%s-%s-%x", provider, region, name)
}
