import React from "react";
import { useForm } from "react-hook-form";

import { Config } from "../../../apis/strims/autoseed/v1/autoseed";
import { Button, ButtonSet, InputError, ToggleInput } from "../../../components/Form";
import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import ForwardLink from "../ForwardLink";

interface AutoseedConfigFormData {
  enable: boolean;
}

const AutoseedConfigForm = () => {
  const [setConfigRes, setConfig] = useLazyCall("autoseed", "setConfig");

  const { handleSubmit, reset, control, formState } = useForm<AutoseedConfigFormData>({
    mode: "onBlur",
    defaultValues: {
      enable: false,
    },
  });

  const setValues = ({ config }: { config?: Config }) =>
    reset(
      {
        enable: config.enable,
      },
      {
        keepDirty: false,
        keepIsValid: false,
      }
    );

  useCall("autoseed", "getConfig", { onComplete: (res) => setValues(res) });

  const onSubmit = handleSubmit(async (data) => {
    const res = await setConfig({
      config: {
        enable: data.enable,
      },
    });
    setValues(res);
  });

  return (
    <>
      <TableTitleBar label="Autoseed" />
      <form className="thing_form" onSubmit={onSubmit}>
        {setConfigRes.error && (
          <InputError error={setConfigRes.error.message || "Error saving ingress settings"} />
        )}
        <ToggleInput control={control} label="Enable" name="enable" />
        <ButtonSet>
          <Button disabled={formState.isSubmitting || !formState.isDirty}>Save Changes</Button>
        </ButtonSet>
        <ForwardLink
          to={`/settings/autoseed/rules`}
          title="Autoseed rules"
          description="Some description of autoseed rules..."
        />
      </form>
    </>
  );
};

export default AutoseedConfigForm;
