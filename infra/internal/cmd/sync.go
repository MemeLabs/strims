package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

func init() {
	rootCmd.AddCommand(syncCmd)
}

var syncCmd = &cobra.Command{
	Use:   "sync",
	Short: "Sync node stats between Prometheus and DB",
	Args:  cobra.NoArgs,
	RunE: func(cmd *cobra.Command, args []string) error {
		if err := backend.SyncNodeStats(cmd.Context()); err != nil {
			return fmt.Errorf("syncing node stats failed: %w", err)
		}

		return nil
	},
}
