package main

import (
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"io/ioutil"
	"log"
	"os"

	"github.com/MemeLabs/go-ppspp/funding/pkg/paypal"
	"go.uber.org/zap"
)

type cfg struct {
	Paypal *paypal.Config `json:"paypal"`
}

func main() {
	cfgPath := flag.String("path", "", "path to funding config file")

	listCmd := flag.NewFlagSet("list", flag.ExitOnError)
	listWhat := listCmd.String("what", "", "transactions or subplans")

	deactivateCmd := flag.NewFlagSet("deactivate", flag.ExitOnError)
	deactivateSubplanId := deactivateCmd.String("id", "", "subplan id")

	flag.Parse()

	if len(os.Args) < 2 {
		fmt.Println("expected subcommands")
		os.Exit(1)
	}

	file, err := os.Open(*cfgPath)
	if err != nil {
		log.Fatalln("failed to open cfg file:", cfgPath, err)
	}

	contents, err := ioutil.ReadAll(file)
	if err != nil {
		log.Fatalln("failed to read cfg file:", err)
	}

	config := new(cfg)
	if err := json.Unmarshal(contents, config); err != nil {
		log.Fatalln("failed to unmarshal cfg contents:", err)
	}

	logger, err := zap.NewDevelopment()
	if err != nil {
		log.Fatalln("logger failed:", err)
	}

	pc, err := paypal.NewClient(config.Paypal, logger)
	if err != nil {
		log.Fatalln("creating paypal client failed:", err)
	}

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	switch os.Args[2] {
	case "list":
		listCmd.Parse(os.Args[3:])
		switch *listWhat {
		case "transactions":
			transactions, err := pc.ListTransactions(ctx)
			if err != nil {
				log.Fatalln("failed to list transactions:", err)
			}

			fmt.Println("Transactions")
			fmt.Println("subject | date | amount | ending | available")

			for _, t := range transactions {
				fmt.Printf("%s | %d | %f | %f | %f\n", t.GetSubject(), t.GetDate(), t.GetAmount(), t.GetEnding(), t.GetAvailable())
			}

		case "subplans":
			plans, err := pc.ListSubPlans(ctx)
			if err != nil {
				log.Fatalln("failed to list subplans:", err)
			}

			fmt.Println("Subplans")
			fmt.Println("id | price")
			for k, v := range plans {
				fmt.Printf("%s | %s\n", k, v)
			}
		default:
			fmt.Println("list what? subplans or transactions")
			break
		}
	case "deactivate":
		deactivateCmd.Parse(os.Args[3:])

		if err := pc.DeactivateSubplan(ctx, *deactivateSubplanId); err != nil {
			log.Fatalln(err)
		}

		fmt.Println("deactivated subplan")
	default:
		fmt.Println("invalid command", os.Args[2])
		os.Exit(1)
	}
}
