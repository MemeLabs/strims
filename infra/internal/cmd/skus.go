package cmd

import (
	"context"
	"fmt"
	"strconv"

	"github.com/MemeLabs/go-ppspp/infra/pkg/node"
	"github.com/spf13/cobra"
)

func init() {
	skusCmd.PersistentFlags().StringP("region", "r", "", "provider region")
	rootCmd.AddCommand(skusCmd)
}

var skusCmd = &cobra.Command{
	Use:               "skus [provider] [region]",
	Short:             "List provider SKUs",
	Args:              cobra.MaximumNArgs(2),
	ValidArgsFunction: providerValidArgsFunc,
	RunE: func(cmd *cobra.Command, args []string) error {
		return nil
	},
}

func formatProviderSKUs(ctx context.Context, driver node.Driver, req *node.SKUsRequest) ([][]string, error) {
	skus, err := driver.SKUs(ctx, req)
	if err != nil {
		return nil, fmt.Errorf("Loading SKUs failed: %w", err)
	}

	rows := [][]string{}
	for _, r := range skus {
		var networkCap, priceHourly, priceMonthly string
		if r.NetworkCap != 0 {
			networkCap = strconv.Itoa(r.NetworkCap)
		}
		if r.PriceHourly != nil {
			priceHourly = fmt.Sprintf("%.4f %s", r.PriceHourly.Value, r.PriceHourly.Currency)
		}
		if r.PriceMonthly != nil {
			priceMonthly = fmt.Sprintf("%.4f %s", r.PriceMonthly.Value, r.PriceMonthly.Currency)
		}

		rows = append(rows, []string{
			r.Name,
			strconv.Itoa(r.CPUs),
			strconv.Itoa(r.Memory),
			networkCap,
			strconv.Itoa(r.NetworkSpeed),
			priceMonthly,
			priceHourly,
		})
	}
	return rows, nil
}
