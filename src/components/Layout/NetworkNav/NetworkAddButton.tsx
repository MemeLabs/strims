import { Base64 } from "js-base64";
import React from "react";
import { useHistory } from "react-router-dom";
import usePortal from "react-useportal";

import { CreateServerResponse } from "../../../apis/strims/network/v1/network";
import { certificateRoot } from "../../../lib/certificate";
import AddNetworkModal from "../../AddNetworkModal";

const NetworkAddButton: React.FC<React.ComponentProps<"button">> = ({ children, ...props }) => {
  const { isOpen, openPortal, closePortal, Portal } = usePortal() as {
    isOpen: () => void;
    openPortal: () => void;
    closePortal: () => void;
    Portal: React.ElementType;
  };
  const history = useHistory();

  const handleCreate = (res: CreateServerResponse) => {
    history.push(
      `/directory/${Base64.fromUint8Array(certificateRoot(res.network.certificate).key, true)}`
    );
    closePortal();
  };

  return (
    <>
      <button {...props} onClick={openPortal}>
        {children}
      </button>
      {isOpen && (
        <Portal>
          <AddNetworkModal onCreate={handleCreate} onClose={closePortal} />
        </Portal>
      )}
    </>
  );
};

export default NetworkAddButton;
