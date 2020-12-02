package cmd

import (
	"fmt"

	"github.com/MemeLabs/go-ppspp/infra/pkg/node"
	"github.com/spf13/cobra"
)

func init() {

}

var diffCmd = &cobra.Command{
	Use:   "diff",
	Short: "Find differences between active instances and the database",
	RunE: func(cmd *cobra.Command, args []string) error {

		var liveNodes []*node.Node
		for _, driver := range backend.NodeDrivers {
			res, err := driver.List(cmd.Context(), nil)
			if err != nil {
				return fmt.Errorf("failed to list nodes for %q: %w", driver.Provider(), err)
			}

			liveNodes = append(liveNodes, res...)
		}

		dbNodes, err := backend.ActiveNodes(cmd.Context())
		if err != nil {
			return fmt.Errorf("failed to get active nodes: %w", err)
		}

		_ = dbNodes

		return nil
	},
}
