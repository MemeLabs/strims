/* eslint-disable no-console */

import * as React from "react";
import { useForm } from "react-hook-form";
import { MdClose } from "react-icons/md";

import { InputError, TextInput } from "../components/Form";
import { MainLayout } from "../components/MainLayout";
import { useClient, useLazyCall } from "../contexts/Api";
import { useProfile } from "../contexts/Profile";
import { useTheme } from "../contexts/Theme";
import { CreateNetworkResponse, DirectoryServerEvent, IDirectoryListing } from "../lib/pb";

interface Listing {
  key: string;
  listing: IDirectoryListing;
}

interface AddNetworkModalProps {
  onCreate: (res: CreateNetworkResponse) => void;
  onClose: () => void;
}

interface AddNetworkFormData {
  name: string;
}

const AddNetworkModal: React.FunctionComponent<AddNetworkModalProps> = ({ onCreate, onClose }) => {
  const [{ profile }] = useProfile();

  const [{ error, loading }, createNetwork] = useLazyCall("createNetwork", {
    onComplete: onCreate,
  });
  const { register, handleSubmit, errors } = useForm<AddNetworkFormData>({
    mode: "onBlur",
  });

  const onSubmit = handleSubmit((data) => createNetwork(data));

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
            {error && <InputError error={error.message || "Error creating network"} />}
            <TextInput
              error={errors.name && "Name is required"}
              inputRef={register({ required: true })}
              label="Network name"
              name="name"
              defaultValue={profile ? `${profile.name}'s Network` : ""}
              placeholder="Network name"
              required
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
