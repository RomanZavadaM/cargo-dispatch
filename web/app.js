const log = document.querySelector("#log");
const state = document.querySelector("#connection");

let ws;
let peer;
let controlChannel;
let fileChannel;

function write(value) {
  log.textContent += `${typeof value === "string" ? value : JSON.stringify(value, null, 2)}\n`;
}

function setState(value) {
  state.textContent = value;
  write(value);
}

async function waitForIceGatheringComplete(pc) {
  if (pc.iceGatheringState === "complete") return;
  await new Promise(resolve => {
    const handler = () => {
      if (pc.iceGatheringState === "complete") {
        pc.removeEventListener("icegatheringstatechange", handler);
        resolve();
      }
    };
    pc.addEventListener("icegatheringstatechange", handler);
  });
}

function setupChannel(channel) {
  channel.addEventListener("open", () => {
    write({channel: channel.label, state: "open"});
    if (channel.label === "control") {
      channel.send(JSON.stringify({type: "hello", source: "browser"}));
    }
  });
  channel.addEventListener("message", event => {
    write({channel: channel.label, message: event.data});
  });
  channel.addEventListener("close", () => {
    write({channel: channel.label, state: "closed"});
  });
}

document.querySelector("#connect").addEventListener("click", async () => {
  const trip = document.querySelector("#trip").value.trim();
  const token = document.querySelector("#token").value.trim();
  if (!trip || !token) {
    write("Вкажіть trip session і token");
    return;
  }

  const browserSession = crypto.randomUUID();
  const wsScheme = location.protocol === "https:" ? "wss" : "ws";
  ws = new WebSocket(`${wsScheme}://${location.host}/signal`);

  ws.addEventListener("open", async () => {
    setState("Signaling підключено");
    ws.send(JSON.stringify({
      type: "browser_hello",
      browser_session_id: browserSession,
      trip_session_id: trip,
      access_token: token
    }));

    peer = new RTCPeerConnection({
      iceServers: [{urls: "stun:stun.l.google.com:19302"}]
    });

    peer.addEventListener("connectionstatechange", () => {
      setState(`WebRTC: ${peer.connectionState}`);
    });

    controlChannel = peer.createDataChannel("control");
    fileChannel = peer.createDataChannel("files");
    setupChannel(controlChannel);
    setupChannel(fileChannel);

    const offer = await peer.createOffer();
    await peer.setLocalDescription(offer);
    await waitForIceGatheringComplete(peer);

    ws.send(JSON.stringify({
      type: "offer",
      trip_session_id: trip,
      from: "browser",
      sdp: JSON.stringify(peer.localDescription)
    }));
    write("Offer відправлено");
  });

  ws.addEventListener("message", async event => {
    const message = JSON.parse(event.data);
    write(message);

    if (message.type === "answer" && message.trip_session_id === trip && peer) {
      const answer = JSON.parse(message.sdp);
      await peer.setRemoteDescription(answer);
      write("Answer застосовано");
    }
  });

  ws.addEventListener("close", () => setState("Signaling відключено"));
  ws.addEventListener("error", () => setState("Помилка signaling"));
});

document.querySelector("#position").addEventListener("click", () => {
  if (!navigator.geolocation) {
    write("Geolocation API недоступний");
    return;
  }
  navigator.geolocation.getCurrentPosition(
    p => {
      const payload = {
        type: "position",
        latitude: p.coords.latitude,
        longitude: p.coords.longitude,
        accuracy: p.coords.accuracy,
        observed_at: new Date().toISOString()
      };
      write(payload);
      if (controlChannel?.readyState === "open") {
        controlChannel.send(JSON.stringify(payload));
      }
    },
    e => write({geolocation_error: e.message}),
    {enableHighAccuracy: true, timeout: 15000}
  );
});

document.querySelector("#file").addEventListener("change", event => {
  const file = event.target.files?.[0];
  if (!file) return;

  const metadata = {
    type: "file_selected",
    name: file.name,
    mime_type: file.type,
    size: file.size
  };
  write(metadata);

  if (fileChannel?.readyState === "open") {
    fileChannel.send(JSON.stringify(metadata));
  }
});
