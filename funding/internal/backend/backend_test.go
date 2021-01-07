package backend

import (
	"encoding/json"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
)

func TestUnmarshalPaypalWebhookEvent(t *testing.T) {
	const input = `
  {
   "id":"WH-19973937YW279670F-02S63370HL636500Y",
   "event_version":"1.0",
   "create_time":"2016-04-28T11:29:31Z",
   "resource_type":"Agreement",
   "event_type":"BILLING.SUBSCRIPTION.CREATED",
   "summary":"A billing subscription was created",
   "resource":{
      "id":"I-PE7JWXKGVN0R",
      "shipping_address":{
         "recipient_name":"Cool Buyer",
         "line1":"3rd st",
         "line2":"cool",
         "city":"San Jose",
         "state":"CA",
         "postal_code":"95112",
         "country_code":"US"
      },
      "plan":{
         "curr_code":"USD",
         "links":[

         ],
         "payment_definitions":[
            {
               "type":"TRIAL",
               "frequency":"Month",
               "frequency_interval":"1",
               "amount":{
                  "value":"5.00"
               },
               "cycles":"5",
               "charge_models":[
                  {
                     "type":"TAX",
                     "amount":{
                        "value":"1.00"
                     }
                  },
                  {
                     "type":"SHIPPING",
                     "amount":{
                        "value":"1.00"
                     }
                  }
               ]
            },
            {
               "type":"REGULAR",
               "frequency":"Month",
               "frequency_interval":"1",
               "amount":{
                  "value":"10.00"
               },
               "cycles":"15",
               "charge_models":[
                  {
                     "type":"TAX",
                     "amount":{
                        "value":"2.00"
                     }
                  },
                  {
                     "type":"SHIPPING",
                     "amount":{
                        "value":"1.00"
                     }
                  }
               ]
            }
         ],
         "merchant_preferences":{
            "setup_fee":{
               "value":"0.00"
            },
            "auto_bill_amount":"YES",
            "max_fail_attempts":"21"
         }
      },
      "payer":{
         "payment_method":"paypal",
         "status":"verified",
         "payer_info":{
            "email":"coolbuyer@example.com",
            "first_name":"Cool",
            "last_name":"Buyer",
            "payer_id":"XLHKRXRA4H7QY",
            "shipping_address":{
               "recipient_name":"Cool Buyer",
               "line1":"3rd st",
               "line2":"cool",
               "city":"San Jose",
               "state":"CA",
               "postal_code":"95112",
               "country_code":"US"
            }
         }
      },
      "agreement_details":{
         "outstanding_balance":{
            "value":"0.00"
         },
         "num_cycles_remaining":"5",
         "num_cycles_completed":"0",
         "final_payment_due_date":"2017-11-30T10:00:00Z",
         "failed_payment_count":"0"
      },
      "description":"desc",
      "state":"Pending",
      "links":[
         {
            "href":"https://api.paypal.com/v1/payments/billing-agreements/I-PE7JWXKGVN0R",
            "rel":"self",
            "method":"GET"
         }
      ],
      "start_date":"2016-04-30T07:00:00Z"
   },
   "links":[
      {
         "href":"https://api.paypal.com/v1/notifications/webhooks-events/WH-19973937YW279670F-02S63370HL636500Y",
         "rel":"self",
         "method":"GET"
      },
      {
         "href":"https://api.paypal.com/v1/notifications/webhooks-events/WH-19973937YW279670F-02S63370HL636500Y/resend",
         "rel":"resend",
         "method":"POST"
      }
   ]
}
`
	e := new(SubEventTwo)
	if err := json.Unmarshal([]byte(input), &e); err != nil {
		t.Fatal(err)
	}

	assert.NotEmpty(t, e.Resource.Payer.PaymentMethod, "")

	t.Fatal("pepoban")
}

type SubEventTwo struct {
	ID           string    `json:"id"`
	EventVersion string    `json:"event_version"`
	CreateTime   time.Time `json:"create_time"`
	ResourceType string    `json:"resource_type"`
	EventType    string    `json:"event_type"`
	Summary      string    `json:"summary"`
	Resource     struct {
		ID              string `json:"id"`
		ShippingAddress struct {
			RecipientName string `json:"recipient_name"`
			Line1         string `json:"line1"`
			Line2         string `json:"line2"`
			City          string `json:"city"`
			State         string `json:"state"`
			PostalCode    string `json:"postal_code"`
			CountryCode   string `json:"country_code"`
		} `json:"shipping_address"`
		Plan struct {
			CurrCode           string        `json:"curr_code"`
			Links              []interface{} `json:"links"`
			PaymentDefinitions []struct {
				Type              string `json:"type"`
				Frequency         string `json:"frequency"`
				FrequencyInterval string `json:"frequency_interval"`
				Amount            struct {
					Value string `json:"value"`
				} `json:"amount"`
				Cycles       string `json:"cycles"`
				ChargeModels []struct {
					Type   string `json:"type"`
					Amount struct {
						Value string `json:"value"`
					} `json:"amount"`
				} `json:"charge_models"`
			} `json:"payment_definitions"`
			MerchantPreferences struct {
				SetupFee struct {
					Value string `json:"value"`
				} `json:"setup_fee"`
				AutoBillAmount  string `json:"auto_bill_amount"`
				MaxFailAttempts string `json:"max_fail_attempts"`
			} `json:"merchant_preferences"`
		} `json:"plan"`
		Payer struct {
			PaymentMethod string `json:"payment_method"`
			Status        string `json:"status"`
			PayerInfo     struct {
				Email           string `json:"email"`
				FirstName       string `json:"first_name"`
				LastName        string `json:"last_name"`
				PayerID         string `json:"payer_id"`
				ShippingAddress struct {
					RecipientName string `json:"recipient_name"`
					Line1         string `json:"line1"`
					Line2         string `json:"line2"`
					City          string `json:"city"`
					State         string `json:"state"`
					PostalCode    string `json:"postal_code"`
					CountryCode   string `json:"country_code"`
				} `json:"shipping_address"`
			} `json:"payer_info"`
		} `json:"payer"`
		AgreementDetails struct {
			OutstandingBalance struct {
				Value string `json:"value"`
			} `json:"outstanding_balance"`
			NumCyclesRemaining  string    `json:"num_cycles_remaining"`
			NumCyclesCompleted  string    `json:"num_cycles_completed"`
			FinalPaymentDueDate time.Time `json:"final_payment_due_date"`
			FailedPaymentCount  string    `json:"failed_payment_count"`
		} `json:"agreement_details"`
		Description string `json:"description"`
		State       string `json:"state"`
		Links       []struct {
			Href   string `json:"href"`
			Rel    string `json:"rel"`
			Method string `json:"method"`
		} `json:"links"`
		StartDate time.Time `json:"start_date"`
	} `json:"resource"`
	Links []struct {
		Href   string `json:"href"`
		Rel    string `json:"rel"`
		Method string `json:"method"`
	} `json:"links"`
}

type SubscriberEvent struct {
	ID              string    `json:"id"`
	CreateTime      time.Time `json:"create_time"`
	EventType       string    `json:"event_type"`
	EventVersion    string    `json:"event_version"`
	ResourceType    string    `json:"resource_type"`
	ResourceVersion string    `json:"resource_version"`
	Summary         string    `json:"summary"`
	Resource        struct {
		ID               string    `json:"id"`
		Status           string    `json:"status"`
		StatusUpdateTime time.Time `json:"status_update_time"`
		PlanID           string    `json:"plan_id"`
		StartTime        time.Time `json:"start_time"`
		Quantity         string    `json:"quantity"`
		ShippingAmount   struct {
			CurrencyCode string `json:"currency_code"`
			Value        string `json:"value"`
		} `json:"shipping_amount"`
		Subscriber struct {
			Name struct {
				GivenName string `json:"given_name"`
				Surname   string `json:"surname"`
			} `json:"name"`
			EmailAddress    string `json:"email_address"`
			ShippingAddress struct {
				Name struct {
					FullName string `json:"full_name"`
				} `json:"name"`
				Address struct {
					AddressLine1 string `json:"address_line_1"`
					AddressLine2 string `json:"address_line_2"`
					AdminArea2   string `json:"admin_area_2"`
					AdminArea1   string `json:"admin_area_1"`
					PostalCode   string `json:"postal_code"`
					CountryCode  string `json:"country_code"`
				} `json:"address"`
			} `json:"shipping_address"`
		} `json:"subscriber"`
		AutoRenewal bool `json:"auto_renewal"`
		BillingInfo struct {
			OutstandingBalance struct {
				CurrencyCode string `json:"currency_code"`
				Value        string `json:"value"`
			} `json:"outstanding_balance"`
			CycleExecutions []struct {
				TenureType                  string `json:"tenure_type"`
				Sequence                    int    `json:"sequence"`
				CyclesCompleted             int    `json:"cycles_completed"`
				CyclesRemaining             int    `json:"cycles_remaining"`
				CurrentPricingSchemeVersion int    `json:"current_pricing_scheme_version"`
			} `json:"cycle_executions"`
			LastPayment struct {
				Amount struct {
					CurrencyCode string `json:"currency_code"`
					Value        string `json:"value"`
				} `json:"amount"`
				Time time.Time `json:"time"`
			} `json:"last_payment"`
			NextBillingTime     time.Time `json:"next_billing_time"`
			FinalPaymentTime    time.Time `json:"final_payment_time"`
			FailedPaymentsCount int       `json:"failed_payments_count"`
		} `json:"billing_info"`
		CreateTime time.Time `json:"create_time"`
		UpdateTime time.Time `json:"update_time"`
		Links      []struct {
			Href   string `json:"href"`
			Rel    string `json:"rel"`
			Method string `json:"method"`
		} `json:"links"`
	} `json:"resource"`
	Links []struct {
		Href   string `json:"href"`
		Rel    string `json:"rel"`
		Method string `json:"method"`
	} `json:"links"`
}
