import { Base64 } from "js-base64";
import * as React from "react";
import { Controller, SubmitHandler, useForm } from "react-hook-form";
import { MdChevronLeft, MdChevronRight } from "react-icons/md";
import { Link, Switch } from "react-router-dom";
import Select from "react-select";
import CreatableSelect from "react-select/creatable";
import { useAsync } from "react-use";

import {
  InputError,
  InputLabel,
  TextAreaInput,
  TextInput,
  ToggleInput,
} from "../../components/Form";
import { PrivateRoute } from "../../components/PrivateRoute";
import { useCall, useClient, useLazyCall } from "../../contexts/Api";
import hostRegex from "../../lib/hostRegex";
import { ICertificate, IVideoIngressChannel, IVideoIngressConfig } from "../../lib/pb";

const rootCertificate = (cert: ICertificate): ICertificate =>
  cert.parent ? rootCertificate(cert.parent) : cert;

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

  const {
    register,
    handleSubmit,
    reset,
    control,
    errors,
    formState,
  } = useForm<VideoIngressConfigFormData>({
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

  const setValues = ({ config }: { config?: IVideoIngressConfig }) => {
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
        isDirty: false,
        isValid: true,
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
        <ToggleInput inputRef={register} label="Enable" name="enabled" />
        <TextInput
          error={errors?.serverAddr}
          inputRef={register({
            required: {
              value: true,
              message: "Server address is required",
            },
            pattern: {
              value: hostRegex(),
              message: "Invalid address format",
            },
          })}
          label="Server address"
          description="RTMP server address"
          name="serverAddr"
          placeholder="eg. 127.0.0.1:1935"
          required
        />
        <TextInput
          error={errors?.publicServerAddr}
          inputRef={register({
            pattern: {
              value: hostRegex({
                localhost: false,
                strictPort: false,
              }),
              message: "Invalid address format",
            },
          })}
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
            render={({ onChange, onBlur, value, name }) => (
              <Select
                onChange={onChange}
                onBlur={onBlur}
                value={value}
                name={name}
                isMulti={true}
                placeholder="Select network"
                className="input_select"
                classNamePrefix="react_select"
                options={networkOptions}
              />
            )}
          />
          <InputError error={errors.serviceNetworks} />
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

interface VideoIngressChannelFormData {
  title: string;
  description: string;
  tags: Array<SelectOption>;
  networkKey: SelectOption;
}

interface VideoIngressChannelFormProps {
  data?: VideoIngressChannelFormData;
  onSubmit: SubmitHandler<VideoIngressChannelFormData>;
}

const VideoIngressChannelForm: React.FC<VideoIngressChannelFormProps> = ({ onSubmit }) => {
  const client = useClient();

  const {
    register,
    handleSubmit,
    reset,
    control,
    errors,
    formState,
    watch,
  } = useForm<VideoIngressChannelFormData>({
    mode: "onBlur",
    defaultValues: {
      title: "",
      description: "",
      tags: [],
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
        error={errors?.title}
        inputRef={register({
          required: {
            value: true,
            message: "Title is required",
          },
          maxLength: {
            value: 100,
            message: "Title too long",
          },
        })}
        label="Title"
        placeholder="Title"
        name="title"
        required
      />
      <TextAreaInput
        error={errors?.description}
        inputRef={register({
          required: {
            value: true,
            message: "Description is required",
          },
          maxLength: {
            value: 500,
            message: "Description too long",
          },
        })}
        label="Description"
        placeholder="Description"
        name="description"
        required
      />
      <InputLabel text="Tags">
        <Controller
          name="tags"
          control={control}
          render={({ onChange, onBlur, value, name }) => (
            <CreatableSelect
              onChange={onChange}
              onBlur={onBlur}
              value={value}
              name={name}
              isMulti={true}
              placeholder="Tags"
              className="input_select"
              classNamePrefix="react_select"
            />
          )}
        />
        <InputError error={errors.tags} />
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
          render={({ onChange, onBlur, value, name }) => {
            return (
              <Select
                onChange={onChange}
                onBlur={onBlur}
                value={value}
                name={name}
                placeholder="Select network"
                className="input_select"
                classNamePrefix="react_select"
                options={networkOptions}
              />
            );
          }}
        />
        <InputError error={errors.networkKey} />
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

const VideoIngressChannels = () => {
  const [channelsRes, listChannels] = useCall("videoIngress", "listChannels");
  const [, createChannel] = useLazyCall("videoIngress", "createChannel");
  const [, deleteChannel] = useLazyCall("videoIngress", "deleteChannel", {
    onComplete: listChannels,
  });

  const handleSubmit = React.useCallback(async (data) => {
    await createChannel({
      directoryListingSnippet: {
        title: data.title,
        description: data.description,
        tags: data.tags.map(({ value }) => value),
      },
      networkKey: Base64.toUint8Array(data.networkKey.value),
    });
    listChannels();
  }, []);

  const rows = channelsRes.value?.channels?.map((channel, i) => {
    return (
      <VideoIngressChannelTableItem
        key={channel.id}
        channel={channel}
        onDelete={() => deleteChannel({ id: channel.id })}
      />
    );
  });

  return (
    <>
      <VideoIngressChannelForm onSubmit={handleSubmit} />
      <div className="thing_list">
        <div>Active Channels ({rows?.length || 0})</div>
        {rows}
      </div>
    </>
  );
};

interface VideoIngressChannelTableItemProps {
  channel: IVideoIngressChannel;
  onDelete: () => void;
}

const VideoIngressChannelTableItem = ({ channel, onDelete }: VideoIngressChannelTableItemProps) => {
  const [channelURLRes] = useCall("videoIngress", "getChannelURL", { args: [{ id: channel.id }] });

  return (
    <div className="thing_list__item" key={channel.id}>
      <div>
        <div>{channel.directoryListingSnippet?.title}</div>
        <div>{channel.directoryListingSnippet?.description}</div>
        <div>{channelURLRes.value?.url}</div>
        <div>
          {channel.directoryListingSnippet?.tags.map((tag) => (
            <span>{tag}</span>
          ))}
        </div>
      </div>
      <button className="input input_button" onClick={onDelete}>
        delete
      </button>
      <pre>{JSON.stringify(channel, null, 2)}</pre>
    </div>
  );
};

const VideoIngressPage = () => {
  // const [serversRes, getVideoIngresss] = useCall("chat", "listServers");

  return (
    <main className="network_page">
      <Switch>
        <PrivateRoute path="/settings/video-ingress" exact component={VideoIngressConfigForm} />
        <PrivateRoute
          path="/settings/video-ingress/channels"
          exact
          component={VideoIngressChannels}
        />
      </Switch>
    </main>
  );
};

export default VideoIngressPage;
