// Runs the driver page in a real headless Chrome against a real Control session.
// No packages: the browser is driven over its DevTools protocol.
// Usage: node driver-page-browser-test.mjs <driver-url>
import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const driverUrl = process.argv[2];
if (!driverUrl) throw new Error("driver URL required");
const apiUrl = driverUrl.replace("/driver/", "/api/trip/");
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));

const profile = mkdtempSync(join(tmpdir(), "driver-page-"));
const chrome = spawn(process.env.CHROME || "google-chrome", [
  "--headless=new", "--no-sandbox", "--no-first-run", "--disable-gpu",
  "--remote-debugging-port=0", `--user-data-dir=${profile}`, "about:blank",
], { stdio: "ignore" });

let socket;
let failed = false;
try {
  const portFile = join(profile, "DevToolsActivePort");
  for (let i = 0; i < 200 && !existsSync(portFile); i++) await sleep(100);
  await sleep(200);
  const port = readFileSync(portFile, "utf8").split("\n")[0];
  const target = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: "PUT" })).json();

  socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });
  let nextId = 0;
  const waiting = new Map();
  const exceptions = [];
  socket.addEventListener("message", event => {
    const message = JSON.parse(event.data);
    if (message.method === "Runtime.exceptionThrown") {
      exceptions.push(JSON.stringify(message.params.exceptionDetails));
    }
    const pending = waiting.get(message.id);
    if (!pending) return;
    waiting.delete(message.id);
    if (message.error) pending.reject(new Error(JSON.stringify(message.error)));
    else pending.resolve(message.result);
  });
  const send = (method, params = {}) => new Promise((resolve, reject) => {
    const id = ++nextId;
    waiting.set(id, { resolve, reject });
    socket.send(JSON.stringify({ id, method, params }));
  });
  const evaluate = async expression => {
    const result = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
    if (result.exceptionDetails) throw new Error(`${expression}: ${JSON.stringify(result.exceptionDetails)}`);
    return result.result.value;
  };
  const waitFor = async (what, check, timeout = 15000) => {
    const end = Date.now() + timeout;
    while (Date.now() < end) {
      if (await check()) return;
      await sleep(200);
    }
    throw new Error(`timed out waiting for: ${what}`);
  };
  const pageHas = expression => () => evaluate(expression);
  const trip = async () => (await fetch(apiUrl)).json();
  const completed = async () => (await trip()).points.filter(point => point.completed).length;
  const expect = (condition, what) => {
    if (!condition) throw new Error(`failed: ${what}`);
    console.log(`PASS ${what}`);
  };

  await send("Runtime.enable");
  await send("Page.enable");
  await send("Network.enable");
  // The map tiles belong to a public service and are not part of this test.
  await send("Network.setBlockedURLs", { urls: ["*tile.openstreetmap.org*"] });
  await send("Page.addScriptToEvaluateOnNewDocument", {
    source: 'localStorage.setItem("cargo-dispatch-driver-locale", "uk");',
  });
  await send("Page.navigate", { url: driverUrl });

  await waitFor("route on the page", pageHas('document.querySelectorAll("#points .point").length === 4'));
  await waitFor("connected status", pageHas('document.getElementById("status").textContent === "Підключено до диспетчера"'));
  expect(true, "the page shows the route and the connection state in Ukrainian");
  const text = await evaluate("document.body.innerText");
  expect(text.includes("Позначити виконаною"), "stops offer a clearly named done button");
  expect(/\d(\.\d)? (Б|КБ|МБ)/.test(await evaluate('document.getElementById("documents").innerText')),
    "document sizes use readable units");
  expect(await evaluate('!!document.querySelector(\'#points a[href^="https://www.google.com/maps/dir/"]\')'),
    "stops link to navigation");

  const doneButton = '[...document.querySelectorAll("#points button")].find(b => b.textContent === "Позначити виконаною")';
  const before = await completed();
  await evaluate(`window.confirm = () => false; ${doneButton}.click(); true`);
  await sleep(1500);
  expect((await completed()) === before, "a refused confirmation marks nothing");
  await evaluate(`window.confirm = () => true; ${doneButton}.click(); true`);
  await waitFor("stop completed in Control", async () => (await completed()) === before + 1);
  expect(true, "a confirmed stop is marked done in Control");

  const pointId = (await trip()).points.find(point => !point.completed).point_id;
  await evaluate(`
    const data = new DataTransfer();
    data.items.add(new File(["browser test"], "browser-test.txt", { type: "text/plain" }));
    document.getElementById("file").files = data.files;
    document.getElementById("filePoint").value = "${pointId}";
    document.getElementById("sendFile").click();
    true`);
  await waitFor("file accepted", pageHas('document.getElementById("fileStatus").textContent === "Диспетчер отримав файл"'));
  const sent = (await trip()).documents.find(item => item.original_name === "browser-test.txt");
  expect(sent && sent.direction === "from_driver" && sent.point_id === pointId,
    "a file sent from the page reaches Control attached to the stop");

  await evaluate(`
    window.shownAlerts = [];
    window.alert = message => window.shownAlerts.push(message);
    window.realFetch = window.fetch;
    window.fetch = (url, options) => String(url).includes("/complete/")
      ? Promise.resolve(new Response("", { status: 409 }))
      : window.realFetch(url, options);
    ${doneButton}.click(); true`);
  await waitFor("route changed message", pageHas('window.shownAlerts.length === 1'));
  expect(await evaluate('window.shownAlerts[0].startsWith("Рейс щойно змінився")'),
    "a refused action is explained in words, not as a status code");

  await evaluate('window.fetch = () => Promise.reject(new TypeError("offline")); true');
  await waitFor("offline status", pageHas('document.getElementById("status").textContent.startsWith("Немає зв’язку")'));
  await evaluate("window.fetch = window.realFetch; true");
  await waitFor("status recovers", pageHas(
    'document.getElementById("status").textContent === "Підключено до диспетчера" && document.getElementById("status").className === ""'));
  expect(true, "the connection state turns red when offline and recovers afterwards");

  for (const [locale, label] of [["en", "Mark as done"], ["de", "Als erledigt markieren"], ["ja", "完了にする"], ["zh-Hans", "标记完成"]]) {
    // A block: every evaluation shares one global scope.
    await evaluate(`{
      const select = document.getElementById("languageSelect");
      select.value = "${locale}";
      select.dispatchEvent(new Event("change"));
    } true`);
    await waitFor(`${locale} texts`, pageHas(`document.getElementById("points").innerText.includes("${label}")`));
    expect(true, `the page switches to ${locale}`);
  }

  expect(exceptions.length === 0, `no script errors ${exceptions.join("; ")}`);
  console.log("Driver page browser test: PASS");
} catch (error) {
  failed = true;
  console.error(error);
} finally {
  socket?.close();
  chrome.kill();
}
process.exit(failed ? 1 : 0);
