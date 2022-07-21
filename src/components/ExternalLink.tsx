// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useMemo } from "react";
import createUrlRegExp from "url-regex-safe";

const validUrlPattern = createUrlRegExp({ exact: true, strict: true });

type ExternalLinkProps = React.ComponentProps<"a">;

const ExternalLink: React.FC<ExternalLinkProps> = ({ href, children, ...props }) => {
  const valid = useMemo(() => validUrlPattern.test(href), [href]);

  return valid ? (
    <a target="_blank" rel="nofollow" href={href} {...props}>
      {children}
    </a>
  ) : (
    <span {...props}>{children}</span>
  );
};

export default ExternalLink;
