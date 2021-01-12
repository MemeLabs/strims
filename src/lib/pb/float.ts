let readFloat32: (buf: Uint8Array, pos: number) => number;
let writeFloat32: (buf: Uint8Array, pos: number, v: number) => void;
let readFloat64: (buf: Uint8Array, pos: number) => number;
let writeFloat64: (buf: Uint8Array, pos: number, v: number) => void;

const f32Scratch = new Float32Array([-0]);
const f32ScratchByes = new Uint8Array(f32Scratch.buffer);
const f64Scratch = new Float64Array([-0]);
const f64ScratchByes = new Uint8Array(f64Scratch.buffer);
const littlEndian = f32ScratchByes[3] === 128;

if (littlEndian) {
  readFloat32 = (buf: Uint8Array, pos: number): number => {
    f32ScratchByes[0] = buf[pos];
    f32ScratchByes[1] = buf[pos + 1];
    f32ScratchByes[2] = buf[pos + 2];
    f32ScratchByes[3] = buf[pos + 3];
    return f32Scratch[0];
  };

  writeFloat32 = (buf: Uint8Array, pos: number, v: number) => {
    f32Scratch[0] = v;
    buf[pos] = f32ScratchByes[0];
    buf[pos + 1] = f32ScratchByes[1];
    buf[pos + 2] = f32ScratchByes[2];
    buf[pos + 3] = f32ScratchByes[3];
  };

  readFloat64 = (buf: Uint8Array, pos: number): number => {
    f64ScratchByes[0] = buf[pos];
    f64ScratchByes[1] = buf[pos + 1];
    f64ScratchByes[2] = buf[pos + 2];
    f64ScratchByes[3] = buf[pos + 3];
    f64ScratchByes[4] = buf[pos + 4];
    f64ScratchByes[5] = buf[pos + 5];
    f64ScratchByes[6] = buf[pos + 6];
    f64ScratchByes[7] = buf[pos + 7];
    return f64Scratch[0];
  };

  writeFloat64 = (buf: Uint8Array, pos: number, v: number) => {
    f64Scratch[0] = v;
    buf[pos] = f64ScratchByes[0];
    buf[pos + 1] = f64ScratchByes[1];
    buf[pos + 2] = f64ScratchByes[2];
    buf[pos + 3] = f64ScratchByes[3];
    buf[pos + 4] = f64ScratchByes[4];
    buf[pos + 5] = f64ScratchByes[5];
    buf[pos + 6] = f64ScratchByes[6];
    buf[pos + 7] = f64ScratchByes[7];
  };
} else {
  readFloat32 = (buf: Uint8Array, pos: number): number => {
    f32ScratchByes[3] = buf[pos];
    f32ScratchByes[2] = buf[pos + 1];
    f32ScratchByes[1] = buf[pos + 2];
    f32ScratchByes[0] = buf[pos + 3];
    return f32Scratch[0];
  };

  writeFloat32 = (buf: Uint8Array, pos: number, v: number) => {
    f32Scratch[0] = v;
    buf[pos] = f32ScratchByes[3];
    buf[pos + 1] = f32ScratchByes[2];
    buf[pos + 2] = f32ScratchByes[1];
    buf[pos + 3] = f32ScratchByes[0];
  };

  readFloat64 = (buf: Uint8Array, pos: number): number => {
    f64ScratchByes[7] = buf[pos];
    f64ScratchByes[6] = buf[pos + 1];
    f64ScratchByes[5] = buf[pos + 2];
    f64ScratchByes[4] = buf[pos + 3];
    f64ScratchByes[3] = buf[pos + 4];
    f64ScratchByes[2] = buf[pos + 5];
    f64ScratchByes[1] = buf[pos + 6];
    f64ScratchByes[0] = buf[pos + 7];
    return f64Scratch[0];
  };

  writeFloat64 = (buf: Uint8Array, pos: number, v: number) => {
    f64Scratch[0] = v;
    buf[pos] = f64ScratchByes[7];
    buf[pos + 1] = f64ScratchByes[6];
    buf[pos + 2] = f64ScratchByes[5];
    buf[pos + 3] = f64ScratchByes[4];
    buf[pos + 4] = f64ScratchByes[3];
    buf[pos + 5] = f64ScratchByes[2];
    buf[pos + 6] = f64ScratchByes[1];
    buf[pos + 7] = f64ScratchByes[0];
  };
}

export { readFloat32, writeFloat32, readFloat64, writeFloat64 };
