const log = document.querySelector("#log");
const state = document.querySelector("#connection");

const FILE_CHUNK_SIZE = 12 * 1024;
const FILE_BUFFER_HIGH_WATER = 512 * 1024;
const FILE_BUFFER_LOW_WATER = 128 * 1024;
const SESSION_KEY = "cargo-connectivity-session";
const EVENT_QUEUE_PREFIX = "cargo-connectivity-events:";

let ws;
let peer;
let controlChannel;
let fileChannel;
let activeTrip;
let activeToken;
let incomingFile;
let reconnectTimer;
let shouldReconnect = false;
let resettingPeer = false;

function write(value) {
  log.textContent += `${typeof value === "string" ? value : JSON.stringify(value, null, 2)}\n`;
}

function setState(value) {
  state.textContent = value;
  write(value);
}

function saveSession() {
  if (!activeTrip || !activeToken) return;
  localStorage.setItem(SESSION_KEY, JSON.stringify({
    trip: activeTrip,
    token: activeToken
  }));
}

function restoreSession() {
  const params = new URLSearchParams(location.search);
  const urlTrip = params.get("trip");
  const urlToken = params.get("token");

  if (urlTrip && urlToken) {
    activeTrip = urlTrip;
    activeToken = urlToken;
    saveSession();
  } else {
    try {
      const saved = JSON.parse(localStorage.getItem(SESSION_KEY) || "null");
      activeTrip = saved?.trip || "";
      activeToken = saved?.token || "";
    } catch {
      activeTrip = "";
      activeToken = "";
    }
  }

  document.querySelector("#trip").value = activeTrip || "";
  document.querySelector("#token").value = activeToken || "";
}

function eventQueueKey() {
  return `${EVENT_QUEUE_PREFIX}${activeTrip || "none"}`;
}

function loadEventQueue() {
  try {
    const queue = JSON.parse(localStorage.getItem(eventQueueKey()) || "[]");
    return Array.isArray(queue) ? queue : [];
  } catch {
    return [];
  }
}

function saveEventQueue(queue) {
  localStorage.setItem(eventQueueKey(), JSON.stringify(queue));
}

function enqueueControlEvent(eventType, payload) {
  if (!activeTrip) {
    write("Немає активного рейсу");
    return;
  }

  const event = {
    type: "driver_event",
    event_id: crypto.randomUUID(),
    trip_session_id: activeTrip,
    event_type: eventType,
    created_at: new Date().toISOString(),
    payload
  };

  const queue = loadEventQueue();
  queue.push(event);
  saveEventQueue(queue);
  write({queued_event: event});
  flushControlQueue();
}

function acknowledgeControlEvent(eventId) {
  const queue = loadEventQueue();
  const next = queue.filter(event => event.event_id !== eventId);
  saveEventQueue(next);
  write({event_acknowledged: eventId, pending: next.length});
}

function flushControlQueue() {
  if (controlChannel?.readyState !== "open") return;
  const queue = loadEventQueue();
  for (const event of queue) {
    controlChannel.send(JSON.stringify(event));
  }
  if (queue.length) {
    write({queue_flushed: queue.length});
  }
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
    channel.binaryType = "arraybuffer";
  }

  channel.addEventListener("open", () => {
    write({channel: channel.label, state: "open"});
    if (channel.label === "control") {
      channel.send(JSON.stringify({type: "hello", source: "browser"}));
      flushControlQueue();
    }
  });

  channel.addEventListener("message", async event => {
    if (channel.label === "files") {
      await handleIncomingFileMessage(event);
      return;
    }

    if (typeof event.data === "string") {
      try {
        const message = JSON.parse(event.data);
        if (message.type === "event_ack") {
          acknowledgeControlEvent(message.event_id);
        } else {
          write({channel: channel.label, message});
        }
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

async function handleIncomingFileMessage(event) {
  if (typeof event.data === "string") {
    let message;
    try {
      message = JSON.parse(event.data);
    } catch {
      write({channel: "files", message: event.data});
      return;
    }

    if (message.type === "file_start") {
      incomingFile = {
        fileId: message.file_id,
        name: message.name,
        mimeType: message.mime_type || "application/octet-stream",
        expectedSize: message.size,
        expectedSha256: message.sha256,
        expectedChunks: message.chunk_count,
        receivedChunks: 0,
        receivedSize: 0,
        chunks: []
      };
      write({type: "file_receive_started", name: message.name, size: message.size});
      return;
    }

    if (message.type === "file_end") {
      if (!incomingFile || incomingFile.fileId !== message.file_id) {
        write({file_receive_error: "file_end without matching file_start"});
        return;
      }

      const file = incomingFile;
      incomingFile = null;
      const blob = new Blob(file.chunks, {type: file.mimeType});
      const buffer = await blob.arrayBuffer();
      const actualSha256 = await sha256Hex(buffer);
      const sizeOk = buffer.byteLength === file.expectedSize;
      const hashOk = actualSha256 === file.expectedSha256;
      const chunksOk = file.receivedChunks === file.expectedChunks;
      const ok = sizeOk && hashOk && chunksOk;

      if (ok) {
        addReceivedFile(file.name, blob);
      }

      fileChannel.send(JSON.stringify({
        type: "file_ack",
        file_id: file.fileId,
        received_size: buffer.byteLength,
        sha256: actualSha256,
        ok,
        message: ok
          ? "browser verified received file"
          : `verification failed: size=${sizeOk}, sha256=${hashOk}, chunks=${chunksOk}`
      }));

      write({
        type: "file_receive_complete",
        file_id: file.fileId,
        name: file.name,
        size: buffer.byteLength,
        sha256: actualSha256,
        ok
      });
      return;
    }

    write({channel: "files", message});
    return;
  }

  if (!incomingFile) {
    write({file_receive_error: "binary chunk without file_start"});
    return;
  }

  const chunk = event.data instanceof ArrayBuffer
    ? event.data
    : await event.data.arrayBuffer();
  incomingFile.chunks.push(chunk);
  incomingFile.receivedChunks += 1;
  incomingFile.receivedSize += chunk.byteLength;
}

function addReceivedFile(name, blob) {
  const downloads = document.querySelector("#downloads");
  const link = document.createElement("a");
  link.href = URL.createObjectURL(blob);
  link.download = name;
  link.textContent = `${name} (${blob.size} bytes)`;
  link.style.display = "block";
  downloads.appendChild(link);
}

async function startPeer(trip, servers) {
  resetPeer();

  peer = new RTCPeerConnection({iceServers: servers});
  peer.addEventListener("connectionstatechange", () => {
    setState(`WebRTC: ${peer?.connectionState || "closed"}`);

    if (peer?.connectionState === "failed") {
      forceSignalReconnect();
    } else if (peer?.connectionState === "disconnected") {
      setTimeout(() => {
        if (peer?.connectionState === "disconnected") {
          forceSignalReconnect();
        }
      }, 5000);
    }
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

function resetPeer() {
  if (resettingPeer) return;
  resettingPeer = true;
  try {
    controlChannel?.close();
    fileChannel?.close();
    peer?.close();
  } catch {
    // Best-effort cleanup before reconnect.
  }
  controlChannel = null;
  fileChannel = null;
  peer = null;
  incomingFile = null;
  resettingPeer = false;
}

function scheduleReconnect() {
  if (!shouldReconnect || reconnectTimer) return;
  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    connectSignal();
  }, 2000);
}

function forceSignalReconnect() {
  if (!shouldReconnect) return;
  resetPeer();

  if (ws && (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)) {
    ws.close();
  } else {
    scheduleReconnect();
  }
}

function connectSignal() {
  if (!activeTrip || !activeToken) return;
  if (ws && (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)) return;

  const browserSession = crypto.randomUUID();
  const wsScheme = location.protocol === "https:" ? "wss" : "ws";
  ws = new WebSocket(`${wsScheme}://${location.host}/signal`);

  ws.addEventListener("open", () => {
    setState("Signaling підключено");
    ws.send(JSON.stringify({
      type: "browser_hello",
      browser_session_id: browserSession,
      trip_session_id: activeTrip,
      access_token: activeToken
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

  ws.addEventListener("close", () => {
    setState("Signaling відключено");
    ws = null;
    resetPeer();
    scheduleReconnect();
  });

  ws.addEventListener("error", () => {
    setState("Помилка signaling");
  });
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
  activeTrip = document.querySelector("#trip").value.trim();
  activeToken = document.querySelector("#token").value.trim();

  if (!activeTrip || !activeToken) {
    write("Вкажіть trip session і token");
    return;
  }

  saveSession();
  shouldReconnect = true;
  connectSignal();
});

document.querySelector("#position").addEventListener("click", () => {
  if (!navigator.geolocation) {
    write("Geolocation API недоступний");
    return;
  }

  navigator.geolocation.getCurrentPosition(
    position => {
      enqueueControlEvent("position", {
        latitude: position.coords.latitude,
        longitude: position.coords.longitude,
        accuracy: position.coords.accuracy
      });
    },
    error => write({geolocation_error: error.message}),
    {enableHighAccuracy: true, timeout: 15000}
  );
});

document.querySelector("#status-arrived").addEventListener("click", () => {
  enqueueControlEvent("status_change", {status: "arrived"});
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

restoreSession();
if (activeTrip && activeToken) {
  shouldReconnect = true;
  connectSignal();
}
