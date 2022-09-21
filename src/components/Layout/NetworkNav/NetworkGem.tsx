// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import clsx from "clsx";
import React from "react";

import { Network } from "../../../apis/strims/network/v1/network";
import { Image } from "../../../apis/strims/type/image";
import { useImage } from "../../../hooks/useImage";
import { certificateRoot } from "../../../lib/certificate";
import Badge from "../../Badge";

export interface NetworkGemProps {
  network: Network;
  peerCount: number;
  icon?: Image;
}

const NetworkGem: React.FC<NetworkGemProps> = (props) => {
  const className = clsx({
    "network_nav__link__gem": true,
    "network_nav__link__gem--with_icon": !!props.icon,
    "network_nav__link__gem--without_icon": !props.icon,
    "network_nav__link__gem--no_peers": props.peerCount === 0,
  });

  return props.icon ? (
    <NetworkGemWithIcon className={className} {...props} />
  ) : (
    <NetworkGemWithColor className={className} {...props} />
  );
};

interface NetworkGemImplProps extends NetworkGemProps {
  className: string;
}

const NetworkGemWithIcon: React.FC<NetworkGemImplProps> = ({ className, peerCount, icon }) => (
  <div className={className} style={{ backgroundImage: `url(${useImage(icon)})` }}>
    <Badge count={peerCount} max={500} />
  </div>
);

const NetworkGemWithColor: React.FC<NetworkGemImplProps> = ({ network, className, peerCount }) => (
  <div className={className} style={{ backgroundColor: "green" }}>
    {certificateRoot(network.certificate).subject.substring(0, 1)}
    <Badge count={peerCount} max={500} />
  </div>
);

export default NetworkGem;
