package cmd

import (
	"fmt"
	"log"
	"os"
	"strings"

	be "github.com/MemeLabs/go-ppspp/infra/internal/backend"
	"github.com/MemeLabs/go-ppspp/infra/pkg/node"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
	"go.uber.org/zap"
)

var (
	// Used for flags.
	cfgFile string
	backend *be.Backend

	rootCmd = &cobra.Command{
		Use:   "infra",
		Short: "Strims infra management cli",
	}
)

// Execute executes the root command.
func Execute() error {
	return rootCmd.Execute()
}

func init() {
	cobra.OnInitialize(initConfig)

	rootCmd.PersistentFlags().StringVar(&cfgFile, "config", "", "config file")
	rootCmd.PersistentFlags().Int8P("logLevel", "v", int8(zap.ErrorLevel), "log level")
	viper.BindPFlag("logLevel", rootCmd.PersistentFlags().Lookup("logLevel"))
}

func er(msg interface{}) {
	fmt.Println("Error:", msg)
	os.Exit(1)
}

func initConfig() {
	if cfgFile != "" {
		viper.SetConfigFile(cfgFile)
	} else {
		viper.SetConfigName("infra")
		viper.SetConfigType("yaml")
		viper.AddConfigPath("/etc/strims/")
		viper.AddConfigPath("$HOME/.strims/")
		viper.AddConfigPath(".")
	}

	viper.SetEnvPrefix("STRIMS_")
	viper.AutomaticEnv()

	if err := viper.ReadInConfig(); err != nil {
		log.Println("Error reading config file:", err)
		os.Exit(1)
	}
	fmt.Println("Using config file:", viper.ConfigFileUsed())

	var config be.Config
	viper.Unmarshal(&config)

	if b, err := be.New(config); err != nil {
		log.Println("Error reading config file:", err)
		os.Exit(1)
	} else {
		backend = b
	}
}

func providerValidArgsFunc(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
	var a []string
	for _, d := range backend.NodeDrivers {
		if strings.HasPrefix(d.Provider(), toComplete) {
			a = append(a, d.Provider())
		}
	}

	if len(a) == 0 {
		return nil, cobra.ShellCompDirectiveNoFileComp
	}

	return a, cobra.ShellCompDirectiveDefault
}

func prependProviderColumn(table [][]string, driver node.Driver) [][]string {
	rows := [][]string{}
	for _, r := range table {
		rows = append(rows, append([]string{driver.Provider()}, r...))
	}
	return rows
}
