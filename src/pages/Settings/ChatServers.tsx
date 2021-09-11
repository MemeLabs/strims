import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Base64 } from "js-base64";
import React, { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { MdChevronLeft, MdChevronRight } from "react-icons/md";
import { Link, Redirect, Switch, useHistory, useParams } from "react-router-dom";
import { useAsync } from "react-use";

import {
  Emote,
  EmoteEffect,
  EmoteFileType,
  EmoteImage,
  EmoteScale,
  IEmoteEffect,
  Server,
} from "../../apis/strims/chat/v1/chat";
import {
  CreatableSelectInput,
  ImageInput,
  ImageValue,
  InputError,
  InputLabel,
  SelectInput,
  SelectOption,
  TextAreaInput,
  TextInput,
  ToggleInput,
} from "../../components/Form";
import { PrivateRoute } from "../../components/PrivateRoute";
import { useCall, useClient, useLazyCall } from "../../contexts/FrontendApi";
import { rootCertificate } from "../../lib/certificate";
import jsonutil from "../../lib/jsonutil";

interface BackLinkProps {
  to: string;
  title: string;
  description: string;
}

const BackLink: React.FC<BackLinkProps> = ({ to, title, description }) => (
  <Link className="input_label input_button" to={to}>
    <MdChevronLeft size="28" />
    <div className="input_label__body">
      <div>{title}</div>
      <div>{description}</div>
    </div>
  </Link>
);

interface ChatServerFormData {
  name: string;
  networkKey: SelectOption<string>;
  tags: SelectOption<string>[];
  modifiers: SelectOption<string>[];
}

interface ChatServerFormProps {
  onSubmit: (data: ChatServerFormData) => void;
  error: Error;
  loading: boolean;
  config?: Server;
  indexLinkVisible?: boolean;
}

const ChatServerForm: React.FC<ChatServerFormProps> = ({
  onSubmit,
  error,
  loading,
  config,
  indexLinkVisible,
}) => {
  const client = useClient();

  const { handleSubmit, control, reset } = useForm<ChatServerFormData>({
    mode: "onBlur",
  });

  const { value: networkOptions } = useAsync(async () => {
    const res = await client.network.list();
    return res.networks.map((n) => ({
      value: Base64.fromUint8Array(rootCertificate(n.certificate).key),
      label: n.name,
    }));
  });

  const setValues = (config: Server) => {
    const networkKey = Base64.fromUint8Array(config.networkKey);

    reset(
      {
        name: config.room.name,
        networkKey: networkOptions.find(({ value }) => value === networkKey),
        tags: config.room.tags.map((value) => ({ value, label: value })),
        modifiers: config.room.modifiers.map((value) => ({ value, label: value })),
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

  return (
    <form className="thing_form" onSubmit={handleSubmit(onSubmit)}>
      {error && <InputError error={error.message || "Error creating chat server"} />}
      {indexLinkVisible && (
        <BackLink
          to="/settings/chat-servers"
          title="Servers"
          description="Some description of servers..."
        />
      )}
      <TextInput
        control={control}
        rules={{
          required: {
            value: true,
            message: "Name is required",
          },
        }}
        name="name"
        label="Name"
        placeholder="Enter a chat room name"
      />
      <SelectInput
        control={control}
        rules={{
          required: {
            value: true,
            message: "Network is required",
          },
        }}
        name="networkKey"
        label="Network"
        placeholder="Select network"
        options={networkOptions}
      />
      <CreatableSelectInput control={control} name="tags" label="Tags" placeholder="Tags" />
      <CreatableSelectInput
        control={control}
        name="modifiers"
        label="Modifiers"
        placeholder="Modifiers"
      />
      {config && (
        <Link
          className="input_label input_button"
          to={`/settings/chat-servers/${config.id}/emotes`}
        >
          <div className="input_label__body">
            <div>Emotes</div>
            <div>Some description of emotes...</div>
          </div>
          <MdChevronRight size="28" />
        </Link>
      )}
      <label className="input_label">
        <div className="input_label__body">
          <button className="input input_button" disabled={loading}>
            {config ? "Update Server" : "Create Server"}
          </button>
        </div>
      </label>
    </form>
  );
};

interface ChatServerTableProps {
  servers: Server[];
  onDelete: () => void;
}

const ChatServerTable: React.FC<ChatServerTableProps> = ({ servers, onDelete }) => {
  const [{ error }, deleteChatServer] = useLazyCall("chat", "deleteServer", {
    onComplete: onDelete,
  });

  if (!servers) {
    return null;
  }

  const rows = servers.map((server) => {
    const handleDelete = () => deleteChatServer({ id: server.id });

    return (
      <div className="thing_list__item" key={server.id.toString()}>
        <Link to={`/settings/chat-servers/${server.id}`}>{server.room.name || "no title"}</Link>
        <button className="input input_button" onClick={handleDelete}>
          delete
        </button>
        <pre>{jsonutil.stringify(server)}</pre>
      </div>
    );
  });
  return <div className="thing_list">{rows}</div>;
};

const ChatServerIndexPage: React.FC = () => {
  const [{ loading, value }, getServers] = useCall("chat", "listServers");

  if (loading) {
    return null;
  }
  if (!value?.servers.length) {
    return <Redirect to="/settings/chat-servers/new" />;
  }
  return (
    <>
      <ChatServerTable servers={value.servers} onDelete={() => getServers()} />
      <Link to="/settings/chat-servers/new">Create Server</Link>
    </>
  );
};

const ChatServerCreateFormPage: React.FC = () => {
  const [{ value }] = useCall("chat", "listServers");
  const history = useHistory();
  const [{ error, loading }, createChatServer] = useLazyCall("chat", "createServer", {
    onComplete: (res) => history.replace(`/settings/chat-servers/${res.server.id}`),
  });

  const onSubmit = (data: ChatServerFormData) =>
    createChatServer({
      networkKey: Base64.toUint8Array(data.networkKey.value),
      room: {
        name: data.name,
        tags: data.tags.map(({ value }) => value),
        modifiers: data.modifiers.map(({ value }) => value),
      },
    });

  return (
    <ChatServerForm
      onSubmit={onSubmit}
      error={error}
      loading={loading}
      indexLinkVisible={!!value?.servers.length}
    />
  );
};

const ChatServerEditFormPage: React.FC = () => {
  const { serverId } = useParams<{ serverId: string }>();
  const [getRes] = useCall("chat", "getServer", { args: [{ id: BigInt(serverId) }] });

  const [updateRes, updateChatServer] = useLazyCall("chat", "updateServer");

  const onSubmit = (data: ChatServerFormData) =>
    updateChatServer({
      id: BigInt(serverId),
      networkKey: Base64.toUint8Array(data.networkKey.value),
      room: {
        name: data.name,
        tags: data.tags.map(({ value }) => value),
        modifiers: data.modifiers.map(({ value }) => value),
      },
    });

  return (
    <ChatServerForm
      onSubmit={onSubmit}
      error={getRes.error || updateRes.error}
      loading={getRes.loading || updateRes.loading}
      config={getRes.value?.server}
      indexLinkVisible={true}
    />
  );
};

interface ChatEmoteFormData {
  name: string;
  image: ImageValue;
  scale: {
    value: EmoteScale;
    label: string;
  };
  contributor: string;
  contributorLink: string;
  css: string;
  animated: boolean;
  animationFrameCount: number;
  animationDuration: number;
  animationIterationCount: number;
  animationEndOnFrame: number;
  animationLoopForever: boolean;
  animationAlternateDirection: boolean;
  defaultModifiers: SelectOption<string>[];
}

const scaleOptions = [
  {
    value: EmoteScale.EMOTE_SCALE_1X,
    label: "1x",
  },
  {
    value: EmoteScale.EMOTE_SCALE_2X,
    label: "2x",
  },
  {
    value: EmoteScale.EMOTE_SCALE_4X,
    label: "4x",
  },
];

interface ChatEmoteFormProps {
  onSubmit: (data: ChatEmoteFormData) => void;
  error: Error;
  loading: boolean;
  serverId: bigint;
  values?: ChatEmoteFormData;
  indexLinkVisible: boolean;
}

const ChatEmoteForm: React.FC<ChatEmoteFormProps> = ({
  onSubmit,
  error,
  loading,
  serverId,
  values = {},
  indexLinkVisible,
}) => {
  const { handleSubmit, control, watch } = useForm<ChatEmoteFormData>({
    mode: "onBlur",
    defaultValues: {
      scale: {
        value: EmoteScale.EMOTE_SCALE_1X,
        label: "1x",
      },
      ...values,
    },
  });

  const animated = watch("animated");

  return (
    <form className="thing_form" onSubmit={handleSubmit(onSubmit)}>
      {error && <InputError error={error.message || "Error creating chat server"} />}
      {indexLinkVisible ? (
        <BackLink
          to={`/settings/chat-servers/${serverId}/emotes`}
          title="Emotes"
          description="Some description of emotes..."
        />
      ) : (
        <BackLink
          to={`/settings/chat-servers/${serverId}`}
          title="Server"
          description="Some description of server..."
        />
      )}
      <TextInput
        control={control}
        rules={{
          required: {
            value: true,
            message: "Name is required",
          },
        }}
        name="name"
        label="Name"
        placeholder="Enter a emote name"
      />
      <InputLabel required={true} text="Image" component="div">
        <ImageInput control={control} name="image" />
      </InputLabel>
      <SelectInput control={control} name="scale" label="Scale" options={scaleOptions} />
      <TextInput control={control} label="contributor" name="contributor" />
      <TextInput control={control} label="contributor link" name="contributorLink" />
      <TextAreaInput control={control} label="css" name="css" />
      <ToggleInput control={control} label="animated" name="animated" />
      <TextInput
        control={control}
        label="frame count"
        name="animationFrameCount"
        type="number"
        disabled={!animated}
      />
      <TextInput
        control={control}
        label="duration (ms)"
        name="animationDuration"
        type="number"
        disabled={!animated}
      />
      <TextInput
        control={control}
        label="loops"
        name="animationIterationCount"
        type="number"
        disabled={!animated}
      />
      <TextInput
        control={control}
        label="end on frame"
        name="animationEndOnFrame"
        type="number"
        disabled={!animated}
      />
      <ToggleInput
        control={control}
        label="loop forever"
        name="animationLoopForever"
        disabled={!animated}
      />
      <ToggleInput
        control={control}
        label="alternate directions"
        name="animationAlternateDirection"
        disabled={!animated}
      />
      <CreatableSelectInput
        control={control}
        name="defaultModifiers"
        label="Default modifiers"
        placeholder="Modifiers"
      />
      <label className="input_label">
        <div className="input_label__body">
          <button className="input input_button" disabled={loading}>
            {values ? "Update Emote" : "Create Emote"}
          </button>
        </div>
      </label>
    </form>
  );
};

interface ImageProps {
  src: EmoteImage;
}

const Image: React.FC<ImageProps> = ({ src }) => {
  const [url] = useState(() =>
    URL.createObjectURL(new Blob([src.data], { type: fileTypeToMimeType(src.fileType) }))
  );
  useEffect(() => () => URL.revokeObjectURL(url));

  return <img srcSet={`${url} ${scaleToDOMScale(src.scale)}`} />;
};

interface ChatEmoteTableProps {
  serverId: bigint;
  emotes: Emote[];
  onDelete: () => void;
}

const ChatEmoteTable: React.FC<ChatEmoteTableProps> = ({ serverId, emotes, onDelete }) => {
  const [, deleteChatEmote] = useLazyCall("chat", "deleteEmote", { onComplete: onDelete });

  if (!emotes) {
    return null;
  }

  const rows = emotes.map((emote) => {
    const handleDelete = () => deleteChatEmote({ serverId, id: emote.id });

    return (
      <div className="thing_list__item" key={emote.id.toString()}>
        <Image src={emote.images[0]} />
        <Link to={`/settings/chat-servers/${serverId}/emotes/${emote.id}`}>{emote.name}</Link>
        <button className="input input_button" onClick={handleDelete}>
          delete
        </button>
      </div>
    );
  });
  return (
    <div className="thing_list">
      <BackLink
        to={`/settings/chat-servers/${serverId}`}
        title="Server"
        description="Some description of server..."
      />
      {rows}
    </div>
  );
};

const ChatServerEmotePage: React.FC = () => {
  const { serverId } = useParams<{ serverId: string }>();
  const [{ loading, value }, getEmotes] = useCall("chat", "listEmotes", {
    args: [{ serverId: BigInt(serverId) }],
  });

  if (loading) {
    return null;
  }
  if (!value?.emotes.length) {
    return <Redirect to={`/settings/chat-servers/${serverId}/emotes/new`} />;
  }
  return (
    <>
      <ChatEmoteTable
        serverId={BigInt(serverId)}
        emotes={value.emotes}
        onDelete={() => getEmotes()}
      />
      <Link to={`/settings/chat-servers/${serverId}/emotes/new`}>Create Emote</Link>
    </>
  );
};

const ChatEmoteCreateFormPage: React.FC = () => {
  const { serverId } = useParams<{ serverId: string }>();
  const [{ value }] = useCall("chat", "listEmotes", {
    args: [{ serverId: BigInt(serverId) }],
  });
  const history = useHistory();
  const [{ error, loading }, createChatEmote] = useLazyCall("chat", "createEmote", {
    onComplete: () => history.replace(`/settings/chat-servers/${serverId}/emotes`),
  });

  const onSubmit = (data: ChatEmoteFormData) =>
    createChatEmote({
      serverId: BigInt(serverId),
      ...toEmoteProps(data),
    });

  return (
    <ChatEmoteForm
      onSubmit={onSubmit}
      error={error}
      loading={loading}
      serverId={BigInt(serverId)}
      indexLinkVisible={!!value?.emotes.length}
    />
  );
};

const mimeTypeToFileType = (type: string): EmoteFileType => {
  switch (type) {
    case "image/png":
      return EmoteFileType.FILE_TYPE_PNG;
    case "image/gif":
      return EmoteFileType.FILE_TYPE_GIF;
    default:
      return EmoteFileType.FILE_TYPE_UNDEFINED;
  }
};

const fileTypeToMimeType = (type: EmoteFileType): string => {
  switch (type) {
    case EmoteFileType.FILE_TYPE_PNG:
      return "image/png";
    case EmoteFileType.FILE_TYPE_GIF:
      return "image/gif";
    case EmoteFileType.FILE_TYPE_UNDEFINED:
      return "application/octet-stream";
  }
};

const scaleToDOMScale = (type: EmoteScale): string => {
  switch (type) {
    case EmoteScale.EMOTE_SCALE_1X:
      return "1x";
    case EmoteScale.EMOTE_SCALE_2X:
      return "2x";
    case EmoteScale.EMOTE_SCALE_4X:
      return "4x";
  }
};

const toEmoteProps = (data: ChatEmoteFormData) => {
  const effects: IEmoteEffect[] = [];
  if (data.animated) {
    effects.push({
      effect: {
        spriteAnimation: {
          frameCount: data.animationFrameCount,
          durationMs: data.animationDuration,
          iterationCount: data.animationIterationCount,
          endOnFrame: data.animationEndOnFrame,
          loopForever: data.animationLoopForever,
          alternateDirection: data.animationAlternateDirection,
        },
      },
    });
  }
  if (data.css) {
    effects.push({
      effect: {
        customCss: {
          css: data.css,
        },
      },
    });
  }
  if (data.defaultModifiers.length > 0) {
    effects.push({
      effect: {
        defaultModifiers: {
          modifiers: data.defaultModifiers.map(({ value }) => value),
        },
      },
    });
  }

  return {
    name: data.name,
    contributor: data.contributor && {
      name: data.contributor,
      link: data.contributorLink,
    },
    images: [
      {
        data: Base64.toUint8Array(data.image.data),
        fileType: mimeTypeToFileType(data.image.type),
        height: data.image.height,
        width: data.image.width,
        scale: data.scale.value,
      },
    ],
    effects,
  };
};

const ChatEmoteEditFormPage: React.FC = () => {
  const { serverId, emoteId } = useParams<{ serverId: string; emoteId: string }>();
  const [{ value, ...getRes }] = useCall("chat", "getEmote", { args: [{ id: BigInt(emoteId) }] });

  const [updateRes, updateChatEmote] = useLazyCall("chat", "updateEmote");

  const onSubmit = (data: ChatEmoteFormData) =>
    updateChatEmote({
      serverId: BigInt(serverId),
      id: BigInt(emoteId),
      ...toEmoteProps(data),
    });

  if (getRes.loading) {
    return null;
  }

  const { emote } = value;
  console.log(emote);
  const data: ChatEmoteFormData = {
    name: emote.name,
    image: {
      data: Base64.fromUint8Array(emote.images[0].data),
      type: fileTypeToMimeType(emote.images[0].fileType),
      height: emote.images[0].height,
      width: emote.images[0].width,
    },
    scale: {
      value: emote.images[0].scale,
      label: scaleToDOMScale(emote.images[0].scale),
    },
    contributor: emote.contributor?.name,
    contributorLink: emote.contributor?.link,
    css: "",
    animated: false,
    animationFrameCount: 0,
    animationDuration: 0,
    animationIterationCount: 0,
    animationEndOnFrame: 0,
    animationLoopForever: false,
    animationAlternateDirection: false,
    defaultModifiers: [],
  };

  emote.effects.forEach(({ effect }) => {
    switch (effect.case) {
      case EmoteEffect.EffectCase.CUSTOM_CSS:
        data.css = effect.customCss.css;
        break;
      case EmoteEffect.EffectCase.SPRITE_ANIMATION:
        data.animated = true;
        data.animationFrameCount = effect.spriteAnimation.frameCount;
        data.animationDuration = effect.spriteAnimation.durationMs;
        data.animationIterationCount = effect.spriteAnimation.iterationCount;
        data.animationEndOnFrame = effect.spriteAnimation.endOnFrame;
        data.animationLoopForever = effect.spriteAnimation.loopForever;
        data.animationAlternateDirection = effect.spriteAnimation.alternateDirection;
        break;
      case EmoteEffect.EffectCase.DEFAULT_MODIFIERS:
        data.defaultModifiers = effect.defaultModifiers.modifiers.map((m) => ({
          label: m,
          value: m,
        }));
        break;
    }
  });

  return (
    <ChatEmoteForm
      onSubmit={onSubmit}
      error={getRes.error || updateRes.error}
      loading={getRes.loading || updateRes.loading}
      values={data}
      serverId={BigInt(serverId)}
      indexLinkVisible={true}
    />
  );
};

const ChatServersPage: React.FC = () => (
  <main className="network_page">
    <Switch>
      <PrivateRoute path="/settings/chat-servers" exact component={ChatServerIndexPage} />
      <PrivateRoute path="/settings/chat-servers/new" exact component={ChatServerCreateFormPage} />
      <PrivateRoute
        path="/settings/chat-servers/:serverId"
        exact
        component={ChatServerEditFormPage}
      />
      <PrivateRoute
        path="/settings/chat-servers/:serverId/emotes"
        exact
        component={ChatServerEmotePage}
      />
      <PrivateRoute
        path="/settings/chat-servers/:serverId/emotes/new"
        exact
        component={ChatEmoteCreateFormPage}
      />
      <PrivateRoute
        path="/settings/chat-servers/:serverId/emotes/:emoteId"
        exact
        component={ChatEmoteEditFormPage}
      />
    </Switch>
  </main>
);

export default ChatServersPage;
