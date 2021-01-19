package cmd

import (
	"context"
	"errors"
	"fmt"
	"os"

	"github.com/MemeLabs/go-ppspp/infra/pkg/node"
	"github.com/olekukonko/tablewriter"
	"github.com/spf13/cobra"
	"golang.org/x/sync/errgroup"
)

func init() {
	skusCmd.PersistentFlags().StringP("region", "r", "", "provider region")
	rootCmd.AddCommand(skusCmd)
}

var skusCmd = &cobra.Command{
	Use:               "skus [provider]",
	Short:             "List provider SKUs",
	Args:              cobra.MinimumNArgs(1),
	ValidArgsFunction: providerValidArgsFunc,
	RunE: func(cmd *cobra.Command, args []string) error {
		header := []string{
			"Name",
			"CPUs (cores)",
			"Memory (MB)",
			"Network Cap (GB)",
			"Network Speed (Mbps)",
			"Price Monthly",
			"Price Hourly",
		}
		alignment := []int{
			tablewriter.ALIGN_DEFAULT,
			tablewriter.ALIGN_DEFAULT,
			tablewriter.ALIGN_DEFAULT,
			tablewriter.ALIGN_DEFAULT,
			tablewriter.ALIGN_DEFAULT,
			tablewriter.ALIGN_RIGHT,
			tablewriter.ALIGN_RIGHT,
		}
		var data [][]string

		region, _ := cmd.PersistentFlags().GetString("region")
		if region == "" {
			return errors.New("must provide a region")
		}

		req := &node.SKUsRequest{
			Region: region,
		}

		if len(args) == 1 {
			provider := args[0]
			driver, ok := backend.NodeDrivers[provider]
			if !ok {
				return fmt.Errorf("unsupported provider: %s", provider)
			}

			rows, err := formatProviderSKUs(cmd.Context(), driver, req)
			if err != nil {
				return err
			}
			data = rows
		} else {
			header = append([]string{"Provider"}, header...)
			alignment = append([]int{tablewriter.ALIGN_DEFAULT}, alignment...)

			eg, ctx := errgroup.WithContext(cmd.Context())
			for _, driver := range backend.NodeDrivers {
				driver := driver
				eg.Go(func() error {
					rows, err := formatProviderSKUs(ctx, driver, req)
					if err != nil {
						return err
					}
					data = append(data, prependProviderColumn(rows, driver)...)
					return nil
				})
			}
			if err := eg.Wait(); err != nil {
				return err
			}
		}

		table := tablewriter.NewWriter(os.Stdout)
		table.SetHeader(header)
		table.SetColumnAlignment(alignment)
		table.AppendBulk(data)
		table.Render()

		return nil
	},
}

func formatProviderSKUs(ctx context.Context, driver node.Driver, req *node.SKUsRequest) ([][]string, error) {
	skus, err := driver.SKUs(ctx, req)
	if err != nil {
		return nil, fmt.Errorf("oading SKUs failed: %w", err)
	}

	rows := [][]string{}
	for _, r := range skus {
		var networkCap, priceHourly, priceMonthly string
		if r.NetworkCap != 0 {
			networkCap = fmt.Sprint(r.NetworkCap)
		}
		priceHourly = fmt.Sprintf("$ %.4f", r.PriceHourly.Value)
		priceMonthly = fmt.Sprintf("$ %.4f", r.PriceMonthly.Value)

		rows = append(rows, []string{
			r.Name,
			fmt.Sprint(r.Cpus),
			fmt.Sprint(r.Memory),
			networkCap,
			fmt.Sprint(r.NetworkSpeed),
			priceMonthly,
			priceHourly,
		})
	}
	return rows, nil
}
