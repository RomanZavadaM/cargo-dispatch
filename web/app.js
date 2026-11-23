const log = document.querySelector("#log");
const state = document.querySelector("#connection");
let ws;

function write(value) {
  log.textContent += `${typeof value === "string" ? value : JSON.stringify(value, null, 2)}\n`;
}

document.querySelector("#connect").addEventListener("click", () => {
  const trip = document.querySelector("#trip").value.trim();
  const token = document.querySelector("#token").value.trim();
  const browserSession = crypto.randomUUID();

  ws = new WebSocket(`${location.protocol === "https:" ? "wss" : "ws"}://${location.host}/signal`);
  ws.addEventListener("open", () => {
    state.textContent = "Signaling підключено";
    ws.send(JSON.stringify({
      type: "browser_hello",
      browser_session_id: browserSession,
      trip_session_id: trip,
      access_token: token
    }));
  });
  ws.addEventListener("message", event => write(JSON.parse(event.data)));
  ws.addEventListener("close", () => { state.textContent = "Відключено"; });
});

document.querySelector("#position").addEventListener("click", () => {
  if (!navigator.geolocation) {
    write("Geolocation API недоступний");
    return;
  }
  navigator.geolocation.getCurrentPosition(
    p => write({latitude: p.coords.latitude, longitude: p.coords.longitude, accuracy: p.coords.accuracy}),
    e => write({geolocation_error: e.message}),
    {enableHighAccuracy: true, timeout: 15000}
  );
});

document.querySelector("#file").addEventListener("change", event => {
  const file = event.target.files?.[0];
  if (file) write({file: file.name, type: file.type, size: file.size});
});
