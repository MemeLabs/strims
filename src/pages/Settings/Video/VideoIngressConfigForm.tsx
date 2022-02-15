import { Base64 } from "js-base64";
import React from "react";
import { useForm } from "react-hook-form";
import { MdChevronRight } from "react-icons/md";
import { Link } from "react-router-dom";
import { useAsync } from "react-use";

import { VideoIngressConfig } from "../../../apis/strims/video/v1/ingress";
import {
  Button,
  ButtonSet,
  InputError,
  SelectInput,
  SelectOption,
  TextInput,
  ToggleInput,
} from "../../../components/Form";
import { useCall, useClient, useLazyCall } from "../../../contexts/FrontendApi";
import { certificateRoot } from "../../../lib/certificate";
import hostRegex from "../../../lib/hostRegex";

interface VideoIngressConfigFormData {
  enabled: boolean;
  serverAddr: string;
  publicServerAddr: string;
  serviceNetworks: Array<SelectOption<string>>;
}

const VideoIngressConfigForm = () => {
  const [setConfigRes, setConfig] = useLazyCall("videoIngress", "setConfig");
  const [{ value: config }] = useCall("videoIngress", "getConfig");
  const client = useClient();

  const { handleSubmit, reset, control, formState } = useForm<VideoIngressConfigFormData>({
    mode: "onBlur",
    defaultValues: {
      enabled: false,
      serverAddr: "",
      publicServerAddr: "",
      serviceNetworks: [],
    },
  });

  const { value: networkOptions } = useAsync(async () => {
    const res = await client.network.list();
    return res.networks.map((n) => {
      const certRoot = certificateRoot(n.certificate);
      return {
        value: Base64.fromUint8Array(certRoot.key),
        label: certRoot.subject,
      };
    });
  });

  const setValues = ({ config }: { config?: VideoIngressConfig }) => {
    const configKeys = Object.fromEntries(
      config.serviceNetworkKeys.map((key) => [Base64.fromUint8Array(key), true])
    );

    reset(
      {
        enabled: config.enabled,
        serverAddr: config.serverAddr,
        publicServerAddr: config.publicServerAddr,
        serviceNetworks: networkOptions.filter(({ value }) => configKeys[value]),
      },
      {
        keepDirty: false,
        keepIsValid: false,
      }
    );
  };

  React.useEffect(() => {
    if (networkOptions && config) {
      setValues(config);
    }
  }, [networkOptions, config]);

  const onSubmit = handleSubmit(async (data) => {
    const res = await setConfig({
      config: {
        enabled: data.enabled,
        serverAddr: data.serverAddr,
        publicServerAddr: data.publicServerAddr,
        serviceNetworkKeys: data.serviceNetworks?.map(({ value }) => Base64.toUint8Array(value)),
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
          description="Public address where peers can reach the RTMP server."
          name="publicServerAddr"
          placeholder="ex: ingress.strims.gg"
        />
        <SelectInput
          control={control}
          name="serviceNetworks"
          label="Sharing"
          description="Enable the creation of streams by peers in selected networks."
          placeholder="Select network"
          isMulti={true}
          options={networkOptions}
        />
        <ButtonSet>
          <Button disabled={formState.isSubmitting || !formState.isDirty}>Save Changes</Button>
        </ButtonSet>
        <Link className="input_label input_button" to="/settings/video/channels">
          <div className="input_label__body">
            <div>Channels</div>
            <div>Some description of channels...</div>
          </div>
          <MdChevronRight size="28" />
        </Link>
      </form>
    </>
  );
};

export default VideoIngressConfigForm;
