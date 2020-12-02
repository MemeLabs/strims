package cmd

import (
	"fmt"

	"github.com/MemeLabs/go-ppspp/infra/internal/models"
	"github.com/MemeLabs/go-ppspp/infra/pkg/node"
	"github.com/spf13/cobra"
	"github.com/volatiletech/sqlboiler/v4/queries/qm"
)

func init() {

}

var diffCmd = &cobra.Command{
	Use:   "diff",
	Short: "Find differences between active instances and the database",
	RunE: func(cmd *cobra.Command, args []string) error {

		var liveNodes, dbNodes []*node.Node
		for _, driver := range backend.NodeDrivers {
			res, err := driver.List(cmd.Context(), nil)
			if err != nil {
				return fmt.Errorf("failed to list nodes for %q: %w", driver.Provider(), err)
			}

			liveNodes = append(liveNodes, res...)
		}

		slice, err := models.Nodes(qm.Where("active=?", 1)).All(cmd.Context(), backend.DB)
		if err != nil {
			return err
		}

		for _, n := range slice {
			dbNodes = append(dbNodes, backend.ModelToNode(n))
		}

		return nil
	},
}
