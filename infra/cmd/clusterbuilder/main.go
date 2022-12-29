package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"os"

	be "github.com/MemeLabs/strims/infra/internal/backend"
	"github.com/MemeLabs/strims/infra/pkg/node"
	"github.com/spf13/viper"
	"golang.org/x/sync/errgroup"
	"gopkg.in/yaml.v3"
)

var (
	infraFile  string
	configFile string
	backend    *be.Backend
)

func init() {
	flag.StringVar(&infraFile, "infra-config", "infra.yaml", "provider config file")
	flag.StringVar(&configFile, "config", "config.yaml", "")
	flag.Parse()
}

func main() {
	if infraFile != "" {
		viper.SetConfigFile(infraFile)
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
		log.Println("Error reading config:", err)
		os.Exit(1)
	}

	var config be.Config
	if err := viper.Unmarshal(&config, config.DecoderConfigOptions); err != nil {
		log.Println("Error reading config:", err)
		os.Exit(1)
	}

	if b, err := be.New(config); err != nil {
		log.Fatalf("Error starting backend: %v", err)
	} else {
		backend = b
	}

	ctx := context.Background()

	switch flag.Arg(0) {
	case "create", "build":
		err := create(ctx)
		if err != nil {
			log.Fatal(err)
		}
	case "delete", "destroy":
		err := destroy(ctx)
		if err != nil {
			log.Fatal(err)
		}
	default:
		log.Fatal("provide an action of create/delete")
	}
}

func create(ctx context.Context) error {
	nodes, err := backend.ActiveNodes(ctx)
	if err != nil {
		return err
	}
	if len(nodes) > 0 {
		return fmt.Errorf("cluster already exists")
	}

	data, err := os.ReadFile(configFile)
	if err != nil {
		return err
	}

	var confItems []struct {
		Provider string `yaml:"provider"`
		Region   string `yaml:"region"`
		SKU      string `yaml:"sku"`
		Count    int    `yaml:"count"`
	}
	if err := yaml.Unmarshal(data, &confItems); err != nil {
		return fmt.Errorf("error reading in config items: %v", err)
	}

	eg, egCtx := errgroup.WithContext(ctx)

	controlPlaneCreated := false
	for _, c := range confItems {
		conf := c

		driver, ok := backend.NodeDrivers[conf.Provider]
		if !ok {
			return fmt.Errorf("invalid node provider for %q", conf.Provider)
		}

		if !controlPlaneCreated {
			if err := backend.CreateNode(
				egCtx,
				driver,
				"",
				conf.Region,
				conf.SKU,
				driver.DefaultUser(),
				"",
				node.Hourly,
				node.TypeController,
			); err != nil {
				log.Fatal(err)
			}
			controlPlaneCreated = true
			c.Count--
		}

		for i := 0; i < c.Count; i++ {
			eg.Go(func() error {
				return backend.CreateNode(egCtx, driver, "", conf.Region, conf.SKU, driver.DefaultUser(), "", node.Hourly, node.TypeWorker)
			})
		}
	}

	if err := eg.Wait(); err != nil {
		return err
	}

	if _, err = backend.RunOnController(
		ctx,
		"kubectl apply -k https://github.com/MemeLabs/strims.git/infra/hack/kubernetes/strims",
	); err != nil {
		return err
	}
	return nil
}

func destroy(ctx context.Context) error {
	nodes, err := backend.ActiveNodes(ctx)
	if err != nil {
		return err
	}

	eg, ctx := errgroup.WithContext(ctx)
	for _, n := range nodes {
		node := n
		eg.Go(func() error {
			return backend.DestroyNode(ctx, node.Name)
		})
	}

	if err := eg.Wait(); err != nil {
		return err
	}

	return nil
}
