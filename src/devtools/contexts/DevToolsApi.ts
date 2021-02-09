import { DevToolsClient } from "../../apis/client";
import create from "../../contexts/Api";

export const {
  ClientContext,
  Provider,
  useClient,
  useCall,
  useLazyCall,
} = create<DevToolsClient>();
