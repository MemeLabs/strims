import { Base64 } from "js-base64";
import React, { useContext } from "react";
import { useParams } from "react-router-dom";

import DirectoryGrid from "../components/Directory/Grid";
import { DirectoryContext } from "../contexts/Directory";
import { useClient } from "../contexts/FrontendApi";

const Directory: React.FC = () => {
  const params = useParams<"networkKey">();
  // const [listings, dispatch] = React.useReducer(directoryReducer, []);
  const [directories] = useContext(DirectoryContext);
  // const client = useClient();

  // console.log(directories);

  const listings = Array.from(directories[params.networkKey]?.listings.values() ?? []);

  // React.useEffect(() => {
  //   const networkKey = Base64.toUint8Array(params.networkKey);
  //   const events = client.directory.open({ networkKey });
  //   events.on("data", ({ event }) => dispatch(event));
  //   events.on("close", () => console.log("directory event stream closed"));
  //   return () => events.destroy();
  // }, [params.networkKey]);

  // const handleTestClick = async () => {
  //   const networkKey = Base64.toUint8Array(params.networkKey);
  //   const res = await client.directory.test({ networkKey });
  //   console.log(res);
  // };

  return (
    <div>
      {/* <button onClick={handleTestClick} className="input input_button">
        test
      </button> */}
      <DirectoryGrid listings={listings} networkKey={params.networkKey} />
    </div>
  );
};

export default Directory;
