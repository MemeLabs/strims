// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

package cmd

import (
	"context"
	"fmt"
	"os"
	"strconv"
	"time"

	be "github.com/MemeLabs/strims/infra/internal/backend"
	"github.com/olekukonko/tablewriter"
	"github.com/spf13/cobra"
)

func init() {
	rootCmd.AddCommand(historyCmd)
}

var historyCmd = &cobra.Command{
	Use:   "history",
	Short: "Display a listing of inactive nodes and details",
	Args:  cobra.NoArgs,
	RunE: func(cmd *cobra.Command, args []string) error {
		header := []string{
			"Provider",
			"Name",
			"Memory (MB)",
			"CPUs",
			"Disk (GB)",
			"Networks",
			"Region",
			"SKU",
			"Hourly",
			"Monthly",
			"Cost",
		}
		var data [][]string

		ctx, cancel := context.WithCancel(context.Background())
		defer cancel()

		nodes, err := backend.InactiveNodes(ctx)
		if err != nil {
			return err
		}

		for _, n := range nodes {
			var networks []string
			networks = append(networks, n.Networks.V4...)
			networks = append(networks, n.Networks.V6...)

			duration := time.Unix(0, n.StoppedAt).Sub(time.Unix(0, n.StartedAt))

			data = append(data, []string{
				n.ProviderName,
				n.Name,
				n.Region.Name,
				strconv.Itoa(n.Memory),
				strconv.Itoa(n.CPUs),
				strconv.Itoa(n.Disk),
				fmt.Sprintf("%s", networks),
				n.SKU.Name,
				fmt.Sprint(n.SKU.PriceHourly.Value),
				fmt.Sprint(n.SKU.PriceMonthly.Value),
				fmt.Sprint(be.ComputeCost(n.SKU, duration)),
			})
		}

		table := tablewriter.NewWriter(os.Stdout)
		table.SetHeader(header)
		table.AppendBulk(data)
		table.Render()

		return nil
	},
}
