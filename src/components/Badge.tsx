import React from "react";

export interface BadgeProps {
  count: number;
}

const Badge: React.FC<BadgeProps> = ({ count }) => {
  return <span className="badge">{count.toLocaleString()}</span>;
};

export default Badge;
