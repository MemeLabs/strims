import {useState, useEffect} from 'react';
import {useGetSet} from 'react-use';

const useMediaSource = ({mimeType}) => {
  const [getSourceBuffer, setSourceBuffer] = useGetSet(null);
  const [operations] = useState([]);

  const transform = newOperation => {
    const sourceBuffer = getSourceBuffer();
    const readOnly = sourceBuffer === null || sourceBuffer.updating;

    if (newOperation !== undefined && (operations.length !== 0 || readOnly)) {
      operations.push(newOperation);
      setImmediate(transform);
      return;
    }

    if (readOnly) {
      return;
    }

    const operation = newOperation || operations.shift();
    if (operation === undefined) {
      return;
    }

    try {
      operation(sourceBuffer);
    } catch (e) {
      operations.unshift(operation);
      setImmediate(transform);
    }
  };

  const [mediaSource] = useState(() => {
    const mediaSource = new MediaSource();

    const handleSourceOpen = () => setSourceBuffer(mediaSource.addSourceBuffer(mimeType));
    mediaSource.addEventListener('sourceopen', handleSourceOpen, {once: true});

    return mediaSource;
  }, []);

  useEffect(() => {
    const sourceBuffer = getSourceBuffer();
    if (sourceBuffer === null) {
      return;
    }

    const handleError = e => console.log(e);
    const handleUpdateEnd = () => transform();

    sourceBuffer.addEventListener('error', handleError);
    sourceBuffer.addEventListener('updateend', handleUpdateEnd);

    return () => {
      sourceBuffer.removeEventListener('error', handleError);
      sourceBuffer.removeEventListener('updateend', handleUpdateEnd);
      mediaSource.removeSourceBuffer(sourceBuffer, handleUpdateEnd);
    };
  }, [getSourceBuffer()]);

  const appendBuffer = buf => transform(sourceBuffer => sourceBuffer.appendBuffer(buf));

  const prune = duration => transform(sourceBuffer => {
    const buffered = sourceBuffer && sourceBuffer.buffered;
    if (buffered && buffered.length && buffered.end(0) > duration) {
      const offset = buffered.end(0) - duration;
      sourceBuffer.remove(0, offset);
    }
  });

  return [mediaSource, {appendBuffer, prune, transform}];
};

export default useMediaSource;
