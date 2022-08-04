// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React, { useCallback, useState } from "react";
import { useNavigate } from "react-router-dom";
import usePortal from "use-portal";

import { CreateServerResponse } from "../../../apis/strims/network/v1/network";
import { useLayout } from "../../../contexts/Layout";
import { certificateRoot } from "../../../lib/certificate";
import AddNetworkModal from "../../AddNetworkModal";

const NetworkAddButton: React.FC<React.ComponentProps<"button">> = ({ children, ...props }) => {
  const layout = useLayout();
  const { Portal } = usePortal({ target: layout.root });
  const navigate = useNavigate();

  const [isOpen, setOpen] = useState(false);
  const openPortal = useCallback(() => setOpen(true), []);
  const closePortal = useCallback(() => setOpen(false), []);

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
