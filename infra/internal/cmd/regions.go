// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package cmd

import (
	"context"
	"fmt"
	"os"

	"github.com/MemeLabs/go-ppspp/infra/pkg/node"
	"github.com/olekukonko/tablewriter"
	"github.com/spf13/cobra"
)

func init() {
	rootCmd.AddCommand(regionsCmd)
}

var regionsCmd = &cobra.Command{
	Use:               "regions [provider]",
	Short:             "List provider regions",
	Args:              cobra.MaximumNArgs(1),
	ValidArgsFunction: providerValidArgsFunc,
	RunE: func(cmd *cobra.Command, args []string) error {
		header := []string{
			"Name",
			"City",
			"Lat/Lng",
		}
		var data [][]string

		if len(args) == 1 {
			provider := args[0]
			driver, ok := backend.NodeDrivers[provider]
			if !ok {
				return fmt.Errorf("unsupported provider: %s", provider)
			}

			rows, err := formatProviderRegions(cmd.Context(), driver)
			if err != nil {
				return err
			}
			data = rows
		} else {
			header = append([]string{"Provider"}, header...)
			for _, driver := range backend.NodeDrivers {
				rows, err := formatProviderRegions(cmd.Context(), driver)
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

func formatProviderRegions(ctx context.Context, driver node.Driver) ([][]string, error) {
	regions, err := driver.Regions(ctx, &node.RegionsRequest{})
	if err != nil {
		return nil, fmt.Errorf("loading regions failed: %w", err)
	}

	rows := [][]string{}
	for _, r := range regions {
		rows = append(rows, []string{
			r.Name,
			r.City,
			r.LatLng.String(),
		})
	}
	return rows, nil
}
