declare global {
    interface Window {
        __gobridge__: any;
        Go: any;
    }
}
export default function (wasmChunks: string[]): (baseURI: string, wasmio: unknown) => Promise<any>;
