// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useParams } from "react-router";
import usePortal from "use-portal";

import InviteAuth from "../components/InviteAuth";
import { useLayout } from "../contexts/Layout";

const Invite: React.FC = () => {
  const layout = useLayout();
  const { Portal } = usePortal({ target: layout.root });

  const { code } = useParams<"code">();

  return (
    <Portal>
      <InviteAuth code={code} />
    </Portal>
  );
};

export default Invite;
