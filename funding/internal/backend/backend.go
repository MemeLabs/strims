// Package backend ...
package backend

import (
	"context"
	"database/sql"
	"encoding/json"
	"errors"
	"fmt"
	"io/ioutil"
	"net/http"
	"os"
	"strconv"
	"time"

	"github.com/MemeLabs/go-ppspp/funding/internal/models"
	"github.com/MemeLabs/go-ppspp/funding/pkg/paypal"
	"github.com/MemeLabs/go-ppspp/pkg/pb"
	"github.com/davecgh/go-spew/spew"
	pp "github.com/plutov/paypal/v3"
	"github.com/volatiletech/null/v8"
	"github.com/volatiletech/sqlboiler/v4/boil"
	"github.com/volatiletech/sqlboiler/v4/queries/qm"
	"go.uber.org/zap"

	_ "github.com/lib/pq"
)

type Funding struct {
	logger *zap.Logger
	paypal *paypal.Paypal
	db     *sql.DB

	Summary *pb.FundingSummary
}

type config struct {
	Paypal *paypal.Config `json:"paypal"`
	DBName string         `json:"db_name"`
	DBUser string         `json:"db_user"`
	DBPass string         `json:"db_pass"`
	DBHost string         `json:"db_host"`
	DBPort int            `json:"db_port"`
}

func New(cfgPath string, logger *zap.Logger) (*Funding, error) {
	file, err := os.Open(cfgPath)
	if err != nil {
		return nil, fmt.Errorf("failed to open cfg file: %q %w", cfgPath, err)
	}

	contents, err := ioutil.ReadAll(file)
	if err != nil {
		return nil, fmt.Errorf("failed to read cfg file: %w", err)
	}

	config := new(config)
	if err = json.Unmarshal(contents, config); err != nil {
		return nil, fmt.Errorf("failed to unmarshal cfg contents: %w", err)
	}

	connStr := fmt.Sprintf("host=%s port=%d user=%s password=%s dbname=%s sslmode=disable", config.DBHost, config.DBPort, config.DBUser, config.DBPass, config.DBName)
	db, err := sql.Open("postgres", connStr)
	if err != nil {
		return nil, fmt.Errorf("faied to open db: %w", err)
	}

	pc, err := paypal.NewClient(config.Paypal, logger)
	if err != nil {
		return nil, fmt.Errorf("creating paypal client failed: %w", err)
	}

	boil.SetDB(db)
	f := &Funding{
		logger: logger,
		paypal: pc,
		db:     db,
	}

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	if err = f.seedDB(ctx); err != nil {
		return nil, fmt.Errorf("failed to seed db: %w", err)
	}

	if err = f.LoadSummary(ctx); err != nil {
		return nil, fmt.Errorf("getting summary failed: %w", err)
	}

	return f, nil
}

func (f *Funding) LoadSummary(ctx context.Context) error {
	f.logger.Debug("loading summary")

	transactionsRes, err := models.Transactions(qm.OrderBy(models.TransactionColumns.Date)).AllG(ctx)
	if err != nil {
		return fmt.Errorf("failed to query transactions: %w", err)
	}

	transactions := make([]*pb.FundingTransaction, len(transactionsRes))
	for i, t := range transactionsRes {
		transactions[i] = &pb.FundingTransaction{
			Subject:   t.Subject,
			Note:      t.Note.String,
			Date:      int64(t.Date),
			Amount:    float32(t.Amount),
			Ending:    float32(t.Ending),
			Available: float32(t.Available),
		}
	}

	subplansRes, err := models.Subplans().AllG(ctx)
	if err != nil {
		return fmt.Errorf("failed to query subplans: %w", err)
	}

	subplans := make(map[string]string, len(subplansRes))
	for _, s := range subplansRes {
		subplans[s.PlanID] = s.Price
	}

	var balance struct {
		Available float32 `boil:"available"`
		Date      int64   `boil:"date"`
	}
	if err := models.NewQuery(
		qm.Select(models.TransactionColumns.Available, models.TransactionColumns.Date),
		qm.From(models.TableNames.Transactions),
		qm.OrderBy("date desc"),
		qm.Limit(1),
	).BindG(ctx, &balance); err != nil {
		return fmt.Errorf("failed to query balance: %w", err)
	}

	// lock?
	f.Summary = &pb.FundingSummary{
		Transactions: transactions,
		Subplans:     subplans,
		Balance: &pb.FundingBalance{
			Total: balance.Available,
			AsOf:  balance.Date,
		},
	}

	return nil
}

func (f *Funding) CreateSubPlan(ctx context.Context, price string) (string, error) {
	res, err := f.paypal.CreateSubscriptionPlan(ctx, price)
	if err != nil {
		return "", fmt.Errorf("failed to create subplan: %w", err)
	}

	subplan := models.Subplan{
		PlanID:  res.ID,
		Price:   price,
		Default: false,
	}

	if err := subplan.InsertG(ctx, boil.Infer()); err != nil {
		return "", fmt.Errorf("failed to insert subplan: %w", err)
	}

	return res.ID, nil
}

func (f *Funding) SetupWebhooks(m *http.ServeMux) error {
	webhooks, err := f.paypal.Client.ListWebhooks("")
	if err != nil {
		return err
	}

	if len(webhooks.Webhooks) > 0 {
		return nil
	}

	return nil
}

func (f *Funding) ValidWebhook(r *http.Request, webhookID string) (bool, error) {
	// We can't validate sandbox requests
	if f.paypal.APIBase == pp.APIBaseSandBox {
		return true, nil
	}

	res, err := f.paypal.Client.VerifyWebhookSignature(r, webhookID)
	if err != nil {
		return false, fmt.Errorf("failed to verify signature: %w", err)
	}

	if res.VerificationStatus == "SUCCESS" {
		return true, nil
	}

	return false, nil
}

func (f *Funding) InsertTransaction(body []byte) error {
	et := new(eventType)
	if err := json.Unmarshal(body, &et); err != nil {
		return fmt.Errorf("failed to unmarshal hook event type: %w", err)
	}

	fmt.Println(et.EventType)
	t := new(models.Transaction)
	t.Service = "paypal"
	// https://developer.paypal.com/docs/api-basics/notifications/webhooks/event-names/
	switch et.EventType {
	// case "BILLING.SUBSCRIPTION.ACTIVATED":
	case "PAYMENT.SALE.COMPLETED":
		// auto debit on subscription (or donation?)
		event := new(paymentSaleCompletedEvent)
		if err := json.Unmarshal(body, &event); err != nil {
			return fmt.Errorf("failed to unmarshal hook event: %w", err)
		}

		t.Subject = event.Summary
		t.Note = null.String{}
		t.Date = int(event.CreateTime.Unix())
		t.Currency = event.Resource.Amount.Currency
		total, err := strconv.ParseFloat(event.Resource.Amount.Total, 32)
		if err != nil {
			return fmt.Errorf("failed to parse total amount: %w", err)
		}
		t.Amount = float32(total)
		t.Available = t.Available + t.Amount
	default:
		f.logger.Debug("received unknown webhook event type", zap.String("event_type", et.EventType))
		return errors.New("we don't handle you")
	}

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	if err := t.InsertG(ctx, boil.Infer()); err != nil {
		return fmt.Errorf("failed to insert transaction: %w", err)
	}

	if err := f.LoadSummary(ctx); err != nil {
		f.logger.Debug("failed trying to refresh summary", zap.Error(err))
		return fmt.Errorf("refreshing summary err: %w", err)
	}
	return nil
}

func (f *Funding) seedDB(ctx context.Context) error {
	transactionsRes, err := models.Transactions(qm.OrderBy(models.TransactionColumns.Date)).AllG(ctx)
	if err != nil {
		return fmt.Errorf("failed to query transactions: %w", err)
	}

	subplansRes, err := models.Subplans().AllG(ctx)
	if err != nil {
		return fmt.Errorf("failed to query subplans: %w", err)
	}

	if len(transactionsRes) > 1 || len(subplansRes) > 1 {
		return nil
	}

	ts, err := f.paypal.ListTransactions(ctx)
	if err != nil {
		return fmt.Errorf("failed to list transactions (db seed): %w", err)
	}

	for _, t := range ts {
		nt := models.Transaction{
			Date:      int(t.GetDate()),
			Subject:   t.GetSubject(),
			Note:      null.StringFrom(t.Note),
			Currency:  "USD",
			Amount:    t.GetAmount(),
			Ending:    t.GetEnding(),
			Available: t.GetAvailable(),
			Service:   "paypal",
		}
		if err = nt.InsertG(ctx, boil.Infer()); err != nil {
			f.logger.Error("failed to insert transactionw", zap.Error(err))
		}
	}

	ss, err := f.paypal.ListSubPlans(ctx)
	if err != nil {
		return fmt.Errorf("failed to list subplans for seeding: %w", err)
	}

	for k, v := range ss {
		ns := models.Subplan{
			PlanID:  k,
			Price:   v,
			Default: false,
		}

		if ns.Price == "1.00" {
			ns.Default = true
		}

		if err := ns.InsertG(ctx, boil.Infer()); err != nil {
			spew.Dump(ns)
			return fmt.Errorf("failed to insert subplan: %w", err)
		}

	}
	return nil
}

type eventType struct {
	EventType string `json:"event_type"`
}

type paymentSaleCompletedEvent struct {
	ID           string    `json:"id"`
	EventVersion string    `json:"event_version"`
	CreateTime   time.Time `json:"create_time"`
	ResourceType string    `json:"resource_type"`
	EventType    string    `json:"event_type"`
	Summary      string    `json:"summary"`
	Resource     struct {
		ID         string    `json:"id"`
		CreateTime time.Time `json:"create_time"`
		UpdateTime time.Time `json:"update_time"`
		Amount     struct {
			Total    string `json:"total"`
			Currency string `json:"currency"`
		} `json:"amount"`
		PaymentMode               string    `json:"payment_mode"`
		State                     string    `json:"state"`
		ProtectionEligibility     string    `json:"protection_eligibility"`
		ProtectionEligibilityType string    `json:"protection_eligibility_type"`
		ClearingTime              time.Time `json:"clearing_time"`
		ParentPayment             string    `json:"parent_payment"`
	} `json:"resource"`
}
