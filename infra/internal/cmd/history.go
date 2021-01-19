package cmd

import (
	"context"
	"fmt"
	"os"
	"time"

	be "github.com/MemeLabs/go-ppspp/infra/internal/backend"
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

		nodes, err := backend.ActiveNodes(ctx, false)
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
				fmt.Sprint(n.Sku.Memory),
				fmt.Sprint(n.Sku.Cpus),
				fmt.Sprint(n.Sku.Disk),
				fmt.Sprintf("%s", networks),
				n.Sku.Name,
				fmt.Sprint(n.Sku.PriceHourly.Value),
				fmt.Sprint(n.Sku.PriceMonthly.Value),
				fmt.Sprint(be.ComputeCost(n.Sku, duration)),
			})
		}

		table := tablewriter.NewWriter(os.Stdout)
		table.SetHeader(header)
		table.AppendBulk(data)
		table.Render()

		return nil
	},
}
