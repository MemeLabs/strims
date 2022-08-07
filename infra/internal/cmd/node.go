// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package cmd

import (
	"crypto/rand"
	"errors"
	"fmt"
	"strings"

	"github.com/MemeLabs/strims/infra/pkg/node"
	"github.com/spf13/cobra"
)

var nodeType node.NodeType

var nodeCmd = &cobra.Command{
	Use:   "node",
	Short: "Manage cluster nodes",
}

var createNodeCmd = &cobra.Command{
	Aliases: []string{"provision", "new"},
	Use:     "create",
	Short:   "Create a node and add it to the cluster",
	RunE: func(cmd *cobra.Command, _ []string) error {
		provider, _ := cmd.Flags().GetString("provider")
		driver, ok := backend.NodeDrivers[provider]
		if !ok {
			return fmt.Errorf("invalid node provider for %q", provider)
		}

		hostname, err := cmd.Flags().GetString("hostname")
		if (hostname == "" && provider == "custom") || err != nil {
			return errors.New("must provide a hostname for custom nodes")
		}

		var region, sku, ipv4, user string
		if provider == "custom" {
			user, err = cmd.Flags().GetString("user")
			if user == "" && err != nil {
				return errors.New("must provide a user for custom nodes")
			}

			ipv4, err = cmd.Flags().GetString("address")
			if ipv4 == "" && err != nil {
				return errors.New("must provide a user for custom nodes")
			}
		} else {
			region, err = cmd.Flags().GetString("region")
			if region == "" && err != nil {
				return errors.New("must provide a region")
			}

			sku, err = cmd.Flags().GetString("sku")
			if sku == "" && err != nil {
				return errors.New("must provide a sku for non-custom nodes")
			}

			regions, err := driver.Regions(cmd.Context(), &node.RegionsRequest{})
			if err != nil {
				return fmt.Errorf("failed to get regions for current driver: %w", err)
			}

			if !node.ValidRegion(region, regions) {
				return fmt.Errorf("invalid region for %q", provider)
			}

			skus, err := driver.SKUs(cmd.Context(), &node.SKUsRequest{Region: region})
			if err != nil {
				return fmt.Errorf("failed to get skus for %q", provider)
			}

			if !node.ValidSKU(sku, skus) {
				return fmt.Errorf("invalid sku for %q", provider)
			}

			user = driver.DefaultUser()
			if hostname == "" {
				hostname = generateHostname(provider, region)
			}
		}

		return backend.CreateNode(cmd.Context(), driver, hostname, region, sku, user, ipv4, node.Hourly, nodeType)
	},
}

var deleteNodeCmd = &cobra.Command{
	Aliases: []string{"delete"},
	Use:     "destroy [name]",
	Short:   "Remove node from cluster and delete by name",
	Args:    cobra.MinimumNArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		return backend.DestroyNode(cmd.Context(), args[0])
	},
}

var listNodeCmd = &cobra.Command{
	Aliases: []string{"get"},
	Use:     "list",
	Short:   "List nodes",
	RunE: func(cmd *cobra.Command, _ []string) error {
		inactive, _ := cmd.Flags().GetBool("inactive")
		var nodes []*node.Node
		var err error
		if inactive {
			nodes, err = backend.InactiveNodes(cmd.Context())
			if err != nil {
				return fmt.Errorf("failed to fetch nodes: %w", err)
			}
		} else {
			nodes, err = backend.ActiveNodes(cmd.Context())
			if err != nil {
				return fmt.Errorf("failed to fetch nodes: %w", err)
			}
		}

		_ = nodes
		return nil
	},
}

var listRegionCmd = &cobra.Command{
	Use:   "regions",
	Short: "List regions supported",
	RunE: func(cmd *cobra.Command, _ []string) error {
		return nil
	},
}

// Generate a lowercased hostname using the provider, region, and random bits
func generateHostname(provider, region string) string {
	name := make([]byte, 4)
	if _, err := rand.Read(name); err != nil {
		panic(err)
	}
	return strings.ToLower(fmt.Sprintf("%s-%s-%x", provider, region, name))
}

func init() {
	createNodeCmd.Flags().VarP(&nodeType, "type", "t", "node type to provision (worker or controller)")
	createNodeCmd.Flags().StringP("provider", "p", "", "hosting provider to use")
	createNodeCmd.Flags().StringP("region", "r", "", "hosting provider region to deploy in")
	createNodeCmd.Flags().StringP("sku", "s", "", "hosting provider sku to provision")
	createNodeCmd.Flags().StringP("hostname", "n", "", "hostname of new node (required for custom, optional otherwise)")
	createNodeCmd.Flags().StringP("address", "a", "", "accessible IPv4 address (custom only)")
	createNodeCmd.Flags().StringP("user", "u", "", "node user to SSH with (custom only)")
	createNodeCmd.MarkFlagRequired("provider")
	createNodeCmd.MarkFlagRequired("type")

	listNodeCmd.Flags().BoolP("inactive", "i", false, "list inactive nodes")
	// listNodeCmd.Flags().StringP("provider", "p", "", "hosting provider to use")

	// TODO: history, diff, regions, skus

	nodeCmd.AddCommand(createNodeCmd)
	nodeCmd.AddCommand(deleteNodeCmd)
	nodeCmd.AddCommand(listNodeCmd)
	rootCmd.AddCommand(nodeCmd)
}
