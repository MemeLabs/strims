import Reader from "./reader";
import Writer from "./writer";

type Message = {
  constructor: (new () => any) & {
    decode(r: Reader | Uint8Array, length?: number): Message;
    encode(m: Message, w?: Writer): Writer;
  };
};

export default Message;
