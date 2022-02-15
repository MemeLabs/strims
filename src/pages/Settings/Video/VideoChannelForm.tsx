import { Base64 } from "js-base64";
import React from "react";
import { SubmitHandler, useForm } from "react-hook-form";
import { MdChevronLeft } from "react-icons/md";
import { Link } from "react-router-dom";
import { useAsync } from "react-use";

import {
  Button,
  ButtonSet,
  CreatableSelectInput,
  SelectInput,
  SelectOption,
  TextAreaInput,
  TextInput,
} from "../../../components/Form";
import { useClient } from "../../../contexts/FrontendApi";
import { certificateRoot } from "../../../lib/certificate";

export interface VideoChannelFormData {
  title: string;
  description: string;
  tags: Array<SelectOption<string>>;
  networkKey: SelectOption<string>;
}

export interface VideoChannelFormProps {
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
    return res.networks.map((n) => {
      const certRoot = certificateRoot(n.certificate);
      return {
        value: Base64.fromUint8Array(certRoot.key),
        label: certRoot.subject,
      };
    });
  });

  return (
    <form className="thing_form" onSubmit={handleSubmit(onSubmit)}>
      <Link className="input_label input_button" to="/settings/video">
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
          maxLength: {
            value: 500,
            message: "Description too long",
          },
        }}
        label="Description"
        placeholder="Description"
        name="description"
      />
      <CreatableSelectInput control={control} name="tags" label="Tags" placeholder="Tags" />
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
      <ButtonSet>
        <Button disabled={formState.isSubmitting || !formState.isDirty}>Save Changes</Button>
      </ButtonSet>
    </form>
  );
};

export default VideoChannelForm;
