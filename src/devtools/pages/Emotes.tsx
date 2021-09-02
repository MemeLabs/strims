import React, { useRef, useState } from "react";
import { useAsync } from "react-use";
import UPNG from "upng-js";

import catJAM from "../assets/catJAM4x.png";
import Nav from "../components/Nav";

interface EmoteProps {
  src: string;
}

const Emote: React.FC<EmoteProps> = () => {
  const canvas = useRef<HTMLCanvasElement>();
  const { value: img } = useAsync(async () => (await fetch(catJAM)).arrayBuffer());
  const [count, setCount] = useState(0);
  const [url, setURL] = useState("");
  const [time, setTime] = useState(0);

  // [x] mirror
  // [x] smol
  // [x] flip
  // [ ] rain
  // [ ] snow
  // [ ] rustle
  // [ ] worth
  // [ ] dank
  // [ ] hyper
  // [ ] love
  // [x] spin
  // [x] wide
  // [ ] virus
  // [ ] banned
  // [ ] lag
  // [x] pause
  // [x] slow
  // [x] fast
  // [x] reverse
  // [ ] jam

  React.useEffect(() => {
    if (!canvas.current || !img) {
      return;
    }

    const width = 112;
    const height = 112;

    // const cw = width * 2 + 40;
    // const ch = height * 2 + 40;

    const cw = width;
    const ch = height;

    canvas.current.width = cw;
    canvas.current.height = ch;

    const ctx = canvas.current.getContext("2d");

    const start = Date.now();
    console.time("render");

    const uimg = UPNG.decode(img);

    console.timeLog("render");

    const imga = new Uint8ClampedArray(UPNG.toRGBA8(uimg)[0]);

    // framestrip
    const frameCount = uimg.width / width;
    const frameSize = imga.byteLength / frameCount;
    const frames = Array.from({ length: frameCount }, () => new Uint8ClampedArray(frameSize));

    for (let y = 0; y < uimg.height; y++) {
      for (let x = 0; x < frameCount; x++) {
        const start = y * uimg.width * 4 + x * width * 4;
        const end = start + width * 4;
        frames[x].set(imga.subarray(start, end), y * width * 4);
      }
    }

    const imgd = frames.map((f) => new ImageData(f, width, uimg.height));

    console.timeLog("render");

    const scratch = document.createElement("canvas");
    scratch.height = height;
    scratch.width = width;
    const sctx = scratch.getContext("2d");

    const paintFrame = (f: number) => {
      sctx.clearRect(0, 0, width, height);
      ctx.clearRect(0, 0, cw, ch);

      // sctx.putImageData(imgd[f], 0, 0);

      // const x = cw / 2 - width / 2;
      // const y = ch / 2 - height / 2;
      // const transformOriginX = 0.5; // the center of sprite width
      // const transformOriginY = 0.5; // the center of sprite height
      // const scaleX = 2;
      // const scaleY = 1;

      // const a = (f / frameCount) * 10 * Math.PI;
      // // const a = 0;
      // const sin = Math.sin(a);
      // const cos = Math.cos(a);
      // const toX = transformOriginX * width;
      // const toY = transformOriginY * height;
      // ctx.setTransform(cos * scaleX, sin * scaleX, -sin * scaleY, cos * scaleY, toX + x, toY + y);

      // ctx.drawImage(scratch, -toX, -toY);

      // ctx.resetTransform();

      ctx.putImageData(imgd[f], 0, 0);
    };

    const out = [];
    const dels = [];
    for (let i = 0; i < frameCount; i++) {
      paintFrame(i);
      out.push(ctx.getImageData(0, 0, cw, ch).data.buffer);
      dels.push(34);
    }

    console.timeLog("render");

    console.log(UPNG);
    const buf = UPNG.encode(out, cw, ch, 0, dels);
    // const buf = UPNG.encodeLL(out, cw, ch, 3, 1, 8, dels);

    console.timeEnd("render");

    const src = URL.createObjectURL(new Blob([buf], { type: "image/png" }));
    setURL(src);

    setTime(Date.now() - start);

    // const now = Date.now();
    // let afr = 0;
    // const paint = () => {
    //   const d = Date.now() - now;
    //   const f = Math.floor(d / 33.3) % frameCount;
    //   paintFrame(f);

    //   afr = requestAnimationFrame(paint);
    // };
    // paint();

    // ctx.fillStyle = "rgba(255, 0, 0, 0.5)";
    // ctx.fillRect(0, 0, 112, 112);
    // return () => cancelAnimationFrame(afr);
  }, [canvas.current, img, count]);

  return (
    <div style={{ backgroundColor: "#111", padding: "10px" }}>
      <canvas ref={canvas} />
      <button onClick={() => setCount(count + 1)}>test</button>
      {url && <img src={url} />}
      {time && <span style={{ color: "white" }}>{time}</span>}
    </div>
  );
};

const Emotes: React.FC = () => {
  return (
    <div>
      <Nav />
      <Emote src={catJAM} />
    </div>
  );
};

export default Emotes;
