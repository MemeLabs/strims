import muxjs from 'mux.js';
import {Buffer} from 'buffer';
import useReady from './useReady';
import useMediaSource from './useMediaSource';

const useSwarmMediaSource = (reader, {
  mimeType = 'video/mp4; codecs="mp4a.40.5,avc1.64001F"',
} = {}) => {
  const [mediaSource, {appendBuffer, prune}] = useMediaSource({
    mimeType,
  });

  useReady(() => {
    const transmuxer = new muxjs.mp4.Transmuxer();

    let initSet = false;
    transmuxer.on('data', event => {
      if (event.type === 'combined') {
        const buf = initSet
          ? event.data
          : Buffer.concat([Buffer.from(event.initSegment), Buffer.from(event.data)]);
        initSet = true;

        appendBuffer(buf);
      } else {
        console.warn('unhandled event', event.type);
      }
    });

    reader.ondata = (n) => transmuxer.push(reader.data.slice(0, n));
    reader.onflush = () => transmuxer.flush();
  }, [reader]);

  return [mediaSource, prune];
};

export default useSwarmMediaSource;
