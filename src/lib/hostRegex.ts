import ipRegex from "ip-regex";
import tlds from "tlds";

const ipv4 = ipRegex.v4().source;
const ipv6 = ipRegex.v6().source;

type Options = {
  localhost?: boolean;
  strictPort?: boolean;
};

export default (options: Options = {}) => {
  options = {
    localhost: true,
    strictPort: true,
    ...options,
  };

  const host = "(?:(?:[a-z\\u00a1-\\uffff0-9][-_]*)*[a-z\\u00a1-\\uffff0-9]+)";
  const domain = "(?:\\.(?:[a-z\\u00a1-\\uffff0-9]-*)*[a-z\\u00a1-\\uffff0-9]+)*";
  const tld = `(?:\\.(?:${tlds.sort((a, b) => b.length - a.length).join("|")}))`;
  const port = "(?::\\d{2,5})";

  let regex = "(?:";
  if (options.localhost) regex += "localhost|";
  regex += `${ipv4}|${ipv6}|${host}|${host}${domain}${tld})`;
  regex += options.strictPort ? port : `${port}?`;

  return new RegExp(regex, "i");
};
