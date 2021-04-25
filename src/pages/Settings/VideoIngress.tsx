import { Base64 } from "js-base64";
import React from "react";
import { Controller, SubmitHandler, useForm } from "react-hook-form";
import { MdChevronLeft, MdChevronRight } from "react-icons/md";
import { Link, Switch } from "react-router-dom";
import Select from "react-select";
import CreatableSelect from "react-select/creatable";
import { useAsync } from "react-use";

import { VideoChannel } from "../../apis/strims/video/v1/channel";
import { VideoIngressConfig } from "../../apis/strims/video/v1/ingress";
import {
  InputError,
  InputLabel,
  TextAreaInput,
  TextInput,
  ToggleInput,
} from "../../components/Form";
import { PrivateRoute } from "../../components/PrivateRoute";
import { useCall, useClient, useLazyCall } from "../../contexts/FrontendApi";
import { rootCertificate } from "../../lib/certificate";
import hostRegex from "../../lib/hostRegex";
import jsonutil from "../../lib/jsonutil";

interface SelectOption {
  value: string;
  label: string;
}

interface VideoIngressConfigFormData {
  enabled: boolean;
  serverAddr: string;
  publicServerAddr: string;
  serviceNetworks: Array<SelectOption>;
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
    return res.networks.map((n) => ({
      value: Base64.fromUint8Array(rootCertificate(n.certificate).key),
      label: n.name,
    }));
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
      <h2>Ingress Server</h2>
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
        <InputLabel
          text="Sharing"
          description="Enable the creation of streams by peers in selected networks."
        >
          <Controller
            name="serviceNetworks"
            control={control}
            render={({ field, fieldState: { error } }) => (
              <>
                <Select
                  {...field}
                  isMulti={true}
                  placeholder="Select network"
                  className="input_select"
                  classNamePrefix="react_select"
                  options={networkOptions}
                />
                <InputError error={error} />
              </>
            )}
          />
        </InputLabel>
        <label className="input_label">
          <div className="input_label__body">
            <button
              className="input input_button"
              disabled={formState.isSubmitting || !formState.isDirty}
            >
              Save Changes
            </button>
          </div>
        </label>
        <Link className="input_label input_button" to="/settings/video-ingress/channels">
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

interface VideoChannelFormData {
  title: string;
  description: string;
  tags: Array<SelectOption>;
  networkKey: SelectOption;
}

interface VideoChannelFormProps {
  data?: VideoChannelFormData;
  onSubmit: SubmitHandler<VideoChannelFormData>;
}

const VideoChannelForm: React.FC<VideoChannelFormProps> = ({ onSubmit }) => {
  const client = useClient();

  const { handleSubmit, control, formState } = useForm<VideoChannelFormData>({
    mode: "onBlur",
    defaultValues: {
      title: "",
      description: "",
      tags: [],
      networkKey: null,
    },
  });

  const { value: networkOptions } = useAsync(async () => {
    const res = await client.network.list();
    return res.networks.map((n) => ({
      value: Base64.fromUint8Array(rootCertificate(n.certificate).key),
      label: n.name,
    }));
  });

  return (
    <form className="thing_form" onSubmit={handleSubmit(onSubmit)}>
      <Link className="input_label input_button" to="/settings/video-ingress">
        <MdChevronLeft size="28" />
        <div className="input_label__body">
          <div>Channels</div>
          <div>Some description of channels...</div>
        </div>
      </Link>

      <TextInput
        control={control}
        rules={{
          required: {
            value: true,
            message: "Title is required",
          },
          maxLength: {
            value: 100,
            message: "Title too long",
          },
        }}
        label="Title"
        placeholder="Title"
        name="title"
      />
      <TextAreaInput
        control={control}
        rules={{
          required: {
            value: true,
            message: "Description is required",
          },
          maxLength: {
            value: 500,
            message: "Description too long",
          },
        }}
        label="Description"
        placeholder="Description"
        name="description"
      />
      <InputLabel text="Tags">
        <Controller
          name="tags"
          control={control}
          render={({ field, fieldState: { error } }) => (
            <>
              <CreatableSelect
                {...field}
                isMulti={true}
                placeholder="Tags"
                className="input_select"
                classNamePrefix="react_select"
              />
              <InputError error={error} />
            </>
          )}
        />
      </InputLabel>
      <InputLabel text="Network">
        <Controller
          name="networkKey"
          control={control}
          rules={{
            required: {
              value: true,
              message: "Network is required",
            },
          }}
          render={({ field, fieldState: { error } }) => (
            <>
              <Select
                {...field}
                placeholder="Select network"
                className="input_select"
                classNamePrefix="react_select"
                options={networkOptions}
              />
              <InputError error={error} />
            </>
          )}
        />
      </InputLabel>
      <label className="input_label">
        <div className="input_label__body">
          <button
            className="input input_button"
            disabled={formState.isSubmitting || !formState.isDirty}
          >
            Save Changes
          </button>
        </div>
      </label>
    </form>
  );
};

const VideoChannels = () => {
  const [channelsRes, listChannels] = useCall("videoChannel", "list");
  const [, createChannel] = useLazyCall("videoChannel", "create");
  const [, deleteChannel] = useLazyCall("videoChannel", "delete", {
    onComplete: listChannels,
  });

  const handleSubmit = React.useCallback(async (data: VideoChannelFormData) => {
    await createChannel({
      directoryListingSnippet: {
        title: data.title,
        description: data.description,
        tags: data.tags.map(({ value }) => value),
      },
      networkKey: Base64.toUint8Array(data.networkKey.value),
    });
    void listChannels();
  }, []);

  const rows = channelsRes.value?.channels?.map((channel) => {
    return (
      <VideoChannelTableItem
        key={channel.id.toString()}
        channel={channel}
        onDelete={() => deleteChannel({ id: channel.id })}
      />
    );
  });

  return (
    <>
      <VideoChannelForm onSubmit={handleSubmit} />
      <div className="thing_list">
        <div>Active Channels ({rows?.length || 0})</div>
        {rows}
      </div>
    </>
  );
};

interface VideoChannelTableItemProps {
  channel: VideoChannel;
  onDelete: () => void;
}

const VideoChannelTableItem = ({ channel, onDelete }: VideoChannelTableItemProps) => {
  const [channelURLRes] = useCall("videoIngress", "getChannelURL", { args: [{ id: channel.id }] });

  return (
    <div className="thing_list__item">
      <div>
        <div>{channel.directoryListingSnippet?.title}</div>
        <div>{channel.directoryListingSnippet?.description}</div>
        <div>{channelURLRes.value?.url}</div>
        <div>
          {channel.directoryListingSnippet?.tags.map((tag, i) => (
            <span key={i}>{tag}</span>
          ))}
        </div>
      </div>
      <button className="input input_button" onClick={onDelete}>
        delete
      </button>
      <pre>{jsonutil.stringify(channel)}</pre>
    </div>
  );
};

const VideoIngressPage = () => {
  // const [serversRes, getVideoIngresss] = useCall("chat", "listServers");

  return (
    <main className="network_page">
      <Switch>
        <PrivateRoute path="/settings/video-ingress" exact component={VideoIngressConfigForm} />
        <PrivateRoute path="/settings/video-ingress/channels" exact component={VideoChannels} />
      </Switch>
    </main>
  );
};

export default VideoIngressPage;
