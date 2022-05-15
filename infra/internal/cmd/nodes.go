// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package cmd

import (
	"context"
	"fmt"
	"os"
	"strconv"

	"github.com/MemeLabs/go-ppspp/infra/pkg/node"
	"github.com/olekukonko/tablewriter"
	"github.com/spf13/cobra"
)

func init() {
	rootCmd.AddCommand(nodesCmd)
}

var nodesCmd = &cobra.Command{
	Use:               "nodes [provider]",
	Short:             "List provider nodes",
	Args:              cobra.MaximumNArgs(1),
	ValidArgsFunction: providerValidArgsFunc,
	RunE: func(cmd *cobra.Command, args []string) error {
		header := []string{
			"ID",
			"Name",
			"Memory (MB)",
			"CPUs",
			"Disk (GB)",
			"Networks",
			"Status",
			"Region",
			"SKU",
		}
		var data [][]string

		if len(args) == 1 {
			provider := args[0]
			driver, ok := backend.NodeDrivers[provider]
			if !ok {
				return fmt.Errorf("unsupported provider: %s", provider)
			}

			rows, err := formatProviderNodes(cmd.Context(), driver)
			if err != nil {
				return err
			}
			data = rows
		} else {
			header = append([]string{"Provider"}, header...)
			for _, driver := range backend.NodeDrivers {
				rows, err := formatProviderNodes(cmd.Context(), driver)
				if err != nil {
					return err
				}
				data = append(data, prependProviderColumn(rows, driver)...)
			}
		}

		table := tablewriter.NewWriter(os.Stdout)
		table.SetHeader(header)
		table.AppendBulk(data)
		table.Render()

		return nil
	},
}

func formatProviderNodes(ctx context.Context, driver node.Driver) ([][]string, error) {
	nodes, err := driver.List(ctx, &node.ListRequest{})
	if err != nil {
		return nil, fmt.Errorf("loading nodes failed: %w", err)
	}

	rows := [][]string{}
	for _, r := range nodes {
		var networks []string
		networks = append(networks, r.Networks.V4...)
		networks = append(networks, r.Networks.V6...)

		rows = append(rows, []string{
			r.ProviderID,
			r.Name,
			strconv.Itoa(r.Memory),
			strconv.Itoa(r.CPUs),
			strconv.Itoa(r.Disk),
			fmt.Sprintf("%s", networks),
			fmt.Sprint(r.Status),
			r.Region.Name,
			r.SKU.Name,
		})
	}
	return rows, nil
}
