// Package paypal ...
package paypal

import (
	"context"
	"errors"
	"fmt"
	"strconv"
	"time"

	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/davecgh/go-spew/spew"
	"github.com/gorilla/mux"
	pp "github.com/plutov/paypal/v3"
	"go.uber.org/zap"
	"golang.org/x/sync/errgroup"
)

type Config struct {
	ClientID     string `json:"client_id"`
	ClientSecret string `json:"client_secret"`
	UseSandbox   bool   `json:"use_sandbox"`
}

type Paypal struct {
	Client  *pp.Client
	logger  *zap.Logger
	APIBase string
}

func NewClient(cfg *Config, logger *zap.Logger) (*Paypal, error) {
	p := new(Paypal)

	if cfg.UseSandbox {
		p.APIBase = pp.APIBaseSandBox
	} else {
		p.APIBase = pp.APIBaseLive
	}

	c, err := pp.NewClient(cfg.ClientID, cfg.ClientSecret, p.APIBase)
	if err != nil {
		return nil, err
	}
	p.Client = c

	_, err = c.GetAccessToken()
	if err != nil {
		return nil, err
	}

	p.logger = logger

	return p, nil
}

// ListTransactions returns the transactions for the last 31 days.
// Invalid request - see details., [{Field:end_date Issue:Date range is greater than 31 days Links:[]}]
func (p *Paypal) ListTransactions(ctx context.Context) ([]*pb.FundingTransaction, error) {
	today := time.Now()
	resp, err := p.Client.ListTransactions(&pp.TransactionSearchRequest{StartDate: today.AddDate(0, 0, -31), EndDate: today})
	if err != nil {
		return nil, fmt.Errorf("failed to call ListTransactions endpoint: %w", err)
	}

	transactions := []*pb.FundingTransaction{}
	for _, tds := range resp.TransactionDetails {
		if tds.TransactionInfo.TransactionStatus != "S" { // not success (pending, refunded, denied, reversed)
			continue
		}
		transactions = append(transactions, newTransaction(tds))
	}

	return transactions, nil
}

func (p *Paypal) GetDefaultProduct(ctx context.Context) (pp.Product, error) {
	const productName = "Strims Video Streaming"
	const productDesc = "P2P Live streaming platform"

	resp, err := p.Client.ListProducts(&pp.ProductListParameters{})
	if err != nil {
		return pp.Product{}, err
	}

	for _, product := range resp.Products {
		if product.Name == productName {
			return product, nil
		}
	}

	product, err := p.Client.CreateProduct(pp.Product{
		Name:        productName,
		Description: productDesc,
		Category:    pp.ProductCategorySoftwareOnlineServices, // TODO: look over list and ensure this is most accurate
		Type:        pp.ProductTypeService,
	})

	return product.Product, err
}

func (p *Paypal) ListSubPlans(ctx context.Context) (map[string]string, error) {
	plans, err := p.Client.ListSubscriptionPlans(&pp.SubscriptionPlanListParameters{})
	if err != nil {
		return nil, err
	}

	if len(plans.Plans) == 0 {
		plan, err := p.CreateSubscriptionPlan(ctx, "1")
		if err != nil {
			return nil, err
		}
		plans.Plans = append(plans.Plans, plan)
	}

	eg := new(errgroup.Group)
	output := make(map[string]string, len(plans.Plans))
	for _, plan := range plans.Plans {
		if plan.Status == pp.SubscriptionPlanStatusActive {
			planid := plan.ID
			eg.Go(func() error {
				subplan, err := p.Client.GetSubscriptionPlan(plan.ID)
				if err != nil {
					return err
				}
				// is this okay to modify here?
				output[planid] = subplan.BillingCycles[0].PricingScheme.FixedPrice.Value
				return nil
			})
		}
	}

	if err := eg.Wait(); err != nil {
		return nil, err
	}

	return output, nil
}

func (p *Paypal) CreateSubscriptionPlan(ctx context.Context, price string) (pp.SubscriptionPlan, error) {
	const defaultSubplan = "Strims Video Streaming Service Plan"
	const subplanDesc = "Basic service plan for Strims"

	product, err := p.GetDefaultProduct(ctx)
	if err != nil {
		return pp.SubscriptionPlan{}, fmt.Errorf("failed to get default product for new plan: %w", err)
	}

	plans, err := p.ListSubPlans(ctx)
	if err != nil {
		return pp.SubscriptionPlan{}, fmt.Errorf("failed to get sub plans: %w", err)
	}

	spew.Dump(plans)

	for _, planPrice := range plans {
		if price == planPrice {
			return pp.SubscriptionPlan{}, errors.New("sub plan already exists for that price")
		}
	}

	plan := pp.SubscriptionPlan{
		ProductId:   product.ID,
		Name:        defaultSubplan,
		Status:      pp.SubscriptionPlanStatusActive,
		Description: subplanDesc,
		BillingCycles: []pp.BillingCycle{{
			PricingScheme: pp.PricingScheme{
				FixedPrice: pp.Money{Value: price, Currency: "USD"},
			},
			Frequency: pp.Frequency{
				IntervalUnit:  "MONTH",
				IntervalCount: 12,
			},
			TenureType: pp.TenureTypeRegular,
			Sequence:   1,
		}},
		Taxes: &pp.Taxes{
			Percentage: "10",
			Inclusive:  false,
		},
		QuantitySupported: false,
		PaymentPreferences: &pp.PaymentPreferences{
			AutoBillOutstanding:   true,
			SetupFee:              &pp.Money{Value: "0", Currency: "USD"},
			SetupFeeFailureAction: pp.SetupFeeFailureActionContinue,
		},
	}

	res, err := p.Client.CreateSubscriptionPlan(plan)
	if err != nil {
		return pp.SubscriptionPlan{}, fmt.Errorf("failed to create subplan: %w", err)
	}

	return res.SubscriptionPlan, nil
}

func (p *Paypal) DeactivateSubplan(ctx context.Context, planid string) error {
	if err := p.Client.DeactivateSubscriptionPlans(planid); err != nil {
		return fmt.Errorf("failed to deactive sub plan(%q): %w", planid, err)
	}

	return nil
}

// TODO Let's handle errors here.
func newTransaction(tds pp.SearchTransactionDetails) *pb.FundingTransaction {
	ts := new(pb.FundingTransaction)
	ts.Subject = tds.TransactionInfo.TransactionSubject
	ts.Note = tds.TransactionInfo.TransactionNote

	ts.Date = time.Time(tds.TransactionInfo.TransactionInitiationDate).Unix()

	tav, err := strconv.ParseFloat(tds.TransactionInfo.TransactionAmount.Value, 32)
	if err != nil {
		return ts
	}
	ts.Amount = float32(tav)

	ebv, err := strconv.ParseFloat(tds.TransactionInfo.EndingBalance.Value, 32)
	if err != nil {
		return ts
	}
	ts.Ending = float32(ebv)

	abv, err := strconv.ParseFloat(tds.TransactionInfo.AvailableBalance.Value, 32)
	if err != nil {
		return ts
	}
	ts.Available = float32(abv)

	return ts
}

func (p *Paypal) SetupWebhooks(r *mux.Router) error {
	webhooks, err := p.Client.ListWebhooks("")
	if err != nil {
		return err
	}

	if len(webhooks.Webhooks) > 0 {
		return nil
	}

	return nil
}
