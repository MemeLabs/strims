import { FrontendClient } from "../apis/client";
import create from "./Api";

export const {
  ClientContext,
  Provider,
  useClient,
  useCall,
  useLazyCall,
} = create<FrontendClient>();
