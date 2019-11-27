/// <reference types="node" />
declare global {
    interface Window {
        __gobridge__: any;
        Go: any;
    }
}
export default function (getBytes: Promise<Buffer>): {};
