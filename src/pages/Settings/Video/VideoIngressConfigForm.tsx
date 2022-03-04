import { Base64 } from "js-base64";
import React from "react";
import { useForm } from "react-hook-form";

import { VideoIngressConfig } from "../../../apis/strims/video/v1/ingress";
import {
  Button,
  ButtonSet,
  InputError,
  NetworkSelectInput,
  TextInput,
  ToggleInput,
} from "../../../components/Form";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import hostRegex from "../../../lib/hostRegex";
import ForwardLink from "../ForwardLink";

interface VideoIngressConfigFormData {
  enabled: boolean;
  serverAddr: string;
  publicServerAddr: string;
  serviceNetworkKeys: string[];
}

const VideoIngressConfigForm = () => {
  const [setConfigRes, setConfig] = useLazyCall("videoIngress", "setConfig");

  const { handleSubmit, reset, control, formState } = useForm<VideoIngressConfigFormData>({
    mode: "onBlur",
    defaultValues: {
      enabled: false,
      serverAddr: "",
      publicServerAddr: "",
      serviceNetworkKeys: [],
    },
  });

  const setValues = ({ config }: { config?: VideoIngressConfig }) =>
    reset(
      {
        enabled: config.enabled,
        serverAddr: config.serverAddr,
        publicServerAddr: config.publicServerAddr,
        serviceNetworkKeys: config.serviceNetworkKeys?.map((value) => Base64.fromUint8Array(value)),
      },
      {
        keepDirty: false,
        keepIsValid: false,
      }
    );

  useCall("videoIngress", "getConfig", { onComplete: (res) => setValues(res) });

  const onSubmit = handleSubmit(async (data) => {
    const res = await setConfig({
      config: {
        enabled: data.enabled,
        serverAddr: data.serverAddr,
        publicServerAddr: data.publicServerAddr,
        serviceNetworkKeys: data.serviceNetworkKeys?.map((value) => Base64.toUint8Array(value)),
      },
    });
    setValues(res);
  });

  return (
    <>
      <form className="thing_form" onSubmit={onSubmit}>
        {setConfigRes.error && (
          <InputError error={setConfigRes.error.message || "Error saving ingress settings"} />
        )}
        <ToggleInput control={control} label="Enable" name="enabled" />
        <TextInput
          control={control}
          rules={{
            required: {
              value: true,
              message: "Server address is required",
            },
            pattern: {
              value: hostRegex(),
              message: "Invalid address format",
            },
          }}
          label="Server address"
          description="RTMP server address"
          name="serverAddr"
          placeholder="eg. 127.0.0.1:1935"
        />
        <TextInput
          control={control}
          rules={{
            pattern: {
              value: hostRegex({
                localhost: false,
                strictPort: false,
              }),
              message: "Invalid address format",
            },
          }}
          label="Public address"
          description="Public address where broadcasters can reach the RTMP server."
          name="publicServerAddr"
          placeholder="ex: ingress.strims.gg"
        />
        <NetworkSelectInput
          control={control}
          name="serviceNetworkKeys"
          label="Sharing"
          description="Enable the creation of streams by peers in selected networks."
          placeholder="Select network"
          isMulti={true}
        />
        <ButtonSet>
          <Button disabled={formState.isSubmitting || !formState.isDirty}>Save Changes</Button>
        </ButtonSet>
        <ForwardLink
          to="/settings/video/channels"
          title="Channels"
          description="Some description of channels..."
        />
      </form>
    </>
  );
};

export default VideoIngressConfigForm;
