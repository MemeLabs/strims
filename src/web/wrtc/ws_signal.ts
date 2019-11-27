
export default function wsSignal(url: string) {
  const ws = new WebSocket(url);

  const offer = new Promise<RTCSessionDescription>((resolve, reject) => {
    ws.onmessage = (e) => resolve(new RTCSessionDescription(JSON.parse(e.data)));
    ws.onerror = (e) => reject(e);
  });

  const sendAnswer = (a) => {
    ws.send(JSON.stringify(a));
    ws.close();
  };

  return {offer, sendAnswer};
}
