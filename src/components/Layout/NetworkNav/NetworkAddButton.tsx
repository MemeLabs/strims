// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React from "react";
import { useNavigate } from "react-router-dom";
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
  const navigate = useNavigate();

  const handleCreate = (res: CreateServerResponse) => {
    navigate(
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
