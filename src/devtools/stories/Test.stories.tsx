import React from "react";
import { Trans } from "react-i18next";

const Translation: React.FC = () => {
  return (
    <>
      <Trans i18nKey="welcome" />
    </>
  );
};

export default [
  {
    name: "foo",
    component: () => <div>foo</div>,
  },
  {
    name: "bar",
    component: () => <div>bar</div>,
  },
  {
    name: "translation",
    component: () => <Translation />,
  },
];
