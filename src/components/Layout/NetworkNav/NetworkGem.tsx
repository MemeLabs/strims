import clsx from "clsx";
import React from "react";

import { Network } from "../../../apis/strims/network/v1/network";
import useObjectURL from "../../../hooks/useObjectURL";
import { certificateRoot } from "../../../lib/certificate";
import Badge from "../../Badge";

export interface NetworkGemProps {
  network: Network;
  peerCount: number;
}

const NetworkGem: React.FC<NetworkGemProps> = ({ network, peerCount }) => {
  const { icon } = network;
  const gemClassName = clsx({
    "network_nav__link__gem": true,
    "network_nav__link__gem--with_icon": !!icon,
    "network_nav__link__gem--without_icon": !icon,
  });

  if (icon) {
    const url = useObjectURL(icon.type, icon.data);
    return (
      <div className={gemClassName} style={{ backgroundImage: `url(${url})` }}>
        <Badge count={peerCount} max={500} />
      </div>
    );
  }

  const backgroundColor = "green";
  return (
    <div className={gemClassName} style={{ backgroundColor }}>
      {certificateRoot(network.certificate).subject.substring(0, 1)}
      <Badge count={peerCount} max={500} />
    </div>
  );
};

export default NetworkGem;
