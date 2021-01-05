import { ButtonsComponentProps, PayPalButtons } from "@paypal/react-paypal-js";
import * as React from "react";
import { Controller, useForm } from "react-hook-form";
import Select from "react-select";
import { useAsync } from "react-use";

import { InputLabel, TextInput, ToggleInput } from "../../components/Form";
import { useClient } from "../contexts/Api";

interface SelectOption {
  value: string;
  label: string;
}

interface SubscribeFormData {
  subAmount: SelectOption;
  is_dontating: boolean;
  custom_price: string | null;
  subplans: Array<SelectOption>;
}

const Home = () => {
  const client = useClient();

  const [isSubscribed, setIsSubscribed] = React.useState(false);

  const { loading, value: resp } = useAsync(() => client.funding.getSummary({}));
  const balance = resp?.summary?.balance;
  const transactions = resp?.summary?.transactions;
  const subplans = Object.entries(resp?.summary?.subplans || {}).map(([k, v]) => ({
    value: k,
    label: v,
  }));

  subplans.push({ value: "custom", label: "custom" });

  const { register, control, errors, handleSubmit, watch, getValues } = useForm<SubscribeFormData>({
    mode: "onBlur",
    defaultValues: {
      is_dontating: false,
      custom_price: null,
    },
  });

  const customPrice = watch("subAmount")?.value === "custom";

  const buttonSubscriptionOptions: ButtonsComponentProps = {
    createSubscription: async (data, actions) => {
      const values = getValues();
      let planId = values.subAmount.value;
      if (planId === "custom") {
        planId = (await client.funding.createSubPlan({ price: values.custom_price })).subPlanId;
      }
      console.log("planid==============", planId);
      actions.subscription.create({
        plan_id: planId,
        application_context: {
          brand_name: "strims",
          locale: "en-US",
          user_action: "SUBSCRIBE_NOW",
          payment_method: {
            payer_selected: "PAYPAL",
            payee_preferred: "IMMEDIATE_PAYMENT_REQUIRED",
          },
        },
      });
    },
    onApprove: (data, actions) => {
      console.log("we just approved some shit!!!");
      console.log(data);
      setIsSubscribed(true);
    },
    onCancel: (data, actions) => {
      console.log("we just canceled some shit!!!");
      console.log(data);
      setIsSubscribed(false);
    },
    onError: (data, actions) => {
      console.log(data);
    },
    style: {
      shape: "pill",
    },
  };

  const onSubmit = handleSubmit(async (data) => {});

  if (loading) {
    return <p>loading</p>;
  }

  return (
    <div>
      <form className="subscription_form" onSubmit={onSubmit}>
        <ToggleInput label="Donate" name="is_donating" inputRef={register} />
        <InputLabel text="Amount" description="Amount to subscribe or donate at.">
          <Controller
            name="subAmount"
            rules={{}}
            control={control}
            render={({ onChange, onBlur, value, name }) => {
              return (
                <Select
                  onChange={onChange}
                  onBlur={onBlur}
                  value={value}
                  name={name}
                  placeholder="Select amount"
                  className="input_select"
                  classNamePrefix="react_select"
                  options={subplans}
                />
              );
            }}
          />
        </InputLabel>
        {customPrice && (
          <TextInput
            error={errors?.custom_price}
            label="Custom amount"
            name="custom_price"
            inputRef={register({ required: true })}
            placeholder="2.00"
            required
          />
        )}
        {!isSubscribed ? <PayPalButtons {...buttonSubscriptionOptions} /> : <p>Woo!</p>}
      </form>
      <p>Current balance: ${balance?.total}</p>
      <div>
        <h3>Transactions</h3>
        {transactions?.map((transaction, idx) => (
          <div key={idx}>
            <p>Subject: "{transaction.subject}"</p>
            <p>Note: "{transaction.note}"</p>
            <p>Amount: ${transaction.amount}</p>
            <p>Date: {new Date(transaction.date * 1000).toISOString()}</p>
          </div>
        ))}
      </div>
    </div>
  );
};

export default Home;
