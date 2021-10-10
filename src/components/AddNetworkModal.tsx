import { Base64 } from "js-base64";
import React from "react";
import { useForm } from "react-hook-form";
import { MdClose } from "react-icons/md";

import { CreateServerResponse } from "../apis/strims/network/v1/network";
import { ImageInput, ImageValue, InputError, TextInput } from "../components/Form";
import { useLazyCall } from "../contexts/FrontendApi";
import { useProfile } from "../contexts/Profile";

interface AddNetworkModalProps {
  onCreate: (res: CreateServerResponse) => void;
  onClose: () => void;
}

interface AddNetworkFormData {
  name: string;
  icon: ImageValue;
}

const AddNetworkModal: React.FC<AddNetworkModalProps> = ({ onCreate, onClose }) => {
  const [{ profile }] = useProfile();

  const [{ error, loading }, createNetwork] = useLazyCall("network", "createServer", {
    onComplete: onCreate,
  });
  const { control, handleSubmit } = useForm<AddNetworkFormData>({
    mode: "onBlur",
  });

  const onSubmit = handleSubmit(({ name, icon: { data, ...icon } }) =>
    createNetwork({
      name,
      icon: {
        data: Base64.toUint8Array(data),
        ...icon,
      },
    })
  );

  return (
    <div className="modal">
      <div className="modal__background" onClick={onClose} />
      <div className="modal__window">
        <button className="modal__close_button" onClick={onClose}>
          <MdClose size={24} />
        </button>
        <form onSubmit={onSubmit}>
          <div className="modal__body">
            <h1 className="create_network__title">Create a network</h1>
            <div className="create_network__description">
              Give your new network a personality with a name and an icon. You can always change it
              later.
            </div>
            {error && <InputError error={error.message || "Error creating network"} />}
            <div className="create_network__avatar">
              <ImageInput name="icon" classNameBase="input_avatar" control={control} />
            </div>
            <TextInput
              control={control}
              rules={{ required: true }}
              label="Network name"
              name="name"
              defaultValue={profile ? `${profile.name}'s Network` : ""}
              placeholder="Network name"
            />
          </div>
          <div className="modal__footer">
            <button className="input input__button" disabled={loading}>
              Create
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

export default AddNetworkModal;
