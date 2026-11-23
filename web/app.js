const log = document.querySelector("#log");
const state = document.querySelector("#connection");

const FILE_CHUNK_SIZE = 12 * 1024;
const FILE_BUFFER_HIGH_WATER = 512 * 1024;
const FILE_BUFFER_LOW_WATER = 128 * 1024;

let ws;
let peer;
let controlChannel;
let fileChannel;
let activeTrip;

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
  if (channel.label === "files") {
    channel.bufferedAmountLowThreshold = FILE_BUFFER_LOW_WATER;
  }

  channel.addEventListener("open", () => {
    write({channel: channel.label, state: "open"});
    if (channel.label === "control") {
      channel.send(JSON.stringify({type: "hello", source: "browser"}));
    }
  });

  channel.addEventListener("message", event => {
    if (typeof event.data === "string") {
      try {
        write({channel: channel.label, message: JSON.parse(event.data)});
      } catch {
        write({channel: channel.label, message: event.data});
      }
    } else {
      write({channel: channel.label, binary_bytes: event.data?.byteLength ?? null});
    }
  });

  channel.addEventListener("close", () => {
    write({channel: channel.label, state: "closed"});
  });
}

async function startPeer(trip, servers) {
  if (peer) return;

  peer = new RTCPeerConnection({iceServers: servers});
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
  write({type: "offer_sent", ice_servers: servers.length});
}

async function sha256Hex(buffer) {
  const digest = await crypto.subtle.digest("SHA-256", buffer);
  return [...new Uint8Array(digest)]
    .map(byte => byte.toString(16).padStart(2, "0"))
    .join("");
}

async function waitForFileBuffer() {
  if (fileChannel.bufferedAmount <= FILE_BUFFER_HIGH_WATER) return;

  await new Promise(resolve => {
    const handler = () => {
      fileChannel.removeEventListener("bufferedamountlow", handler);
      resolve();
    };
    fileChannel.addEventListener("bufferedamountlow", handler, {once: true});
  });
}

async function sendFile(file) {
  if (fileChannel?.readyState !== "open") {
    write("Файловий канал ще не відкритий");
    return;
  }

  const fileId = crypto.randomUUID();
  const buffer = await file.arrayBuffer();
  const sha256 = await sha256Hex(buffer);
  const chunkCount = Math.ceil(buffer.byteLength / FILE_CHUNK_SIZE);

  fileChannel.send(JSON.stringify({
    type: "file_start",
    file_id: fileId,
    name: file.name,
    mime_type: file.type || "application/octet-stream",
    size: file.size,
    sha256,
    chunk_size: FILE_CHUNK_SIZE,
    chunk_count: chunkCount
  }));

  for (let start = 0; start < buffer.byteLength; start += FILE_CHUNK_SIZE) {
    await waitForFileBuffer();
    const end = Math.min(start + FILE_CHUNK_SIZE, buffer.byteLength);
    fileChannel.send(buffer.slice(start, end));
  }

  await waitForFileBuffer();
  fileChannel.send(JSON.stringify({
    type: "file_end",
    file_id: fileId
  }));

  write({
    type: "file_sent",
    file_id: fileId,
    name: file.name,
    size: file.size,
    sha256,
    chunks: chunkCount
  });
}

document.querySelector("#connect").addEventListener("click", () => {
  const trip = document.querySelector("#trip").value.trim();
  const token = document.querySelector("#token").value.trim();
  if (!trip || !token) {
    write("Вкажіть trip session і token");
    return;
  }

  activeTrip = trip;
  const browserSession = crypto.randomUUID();
  const wsScheme = location.protocol === "https:" ? "wss" : "ws";
  ws = new WebSocket(`${wsScheme}://${location.host}/signal`);

  ws.addEventListener("open", () => {
    setState("Signaling підключено");
    ws.send(JSON.stringify({
      type: "browser_hello",
      browser_session_id: browserSession,
      trip_session_id: trip,
      access_token: token
    }));
  });

  ws.addEventListener("message", async event => {
    const message = JSON.parse(event.data);
    write(message);

    if (
      message.type === "ice_config" &&
      message.trip_session_id === activeTrip
    ) {
      await startPeer(activeTrip, message.servers);
      return;
    }

    if (
      message.type === "answer" &&
      message.trip_session_id === activeTrip &&
      peer
    ) {
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

document.querySelector("#file").addEventListener("change", async event => {
  const file = event.target.files?.[0];
  if (!file) return;

  try {
    await sendFile(file);
  } catch (error) {
    write({file_error: error instanceof Error ? error.message : String(error)});
  }
});
