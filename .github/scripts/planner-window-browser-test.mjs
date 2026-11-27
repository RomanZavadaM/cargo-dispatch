// Runs the built Planner window in a real headless Chrome against the real Planner
// service. Only the Tauri bridge is replaced: calls go over HTTP to the test server.
// No packages: the browser is driven over its DevTools protocol.
// Usage: node planner-window-browser-test.mjs <window-url>
import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const windowUrl = process.argv[2];
if (!windowUrl) throw new Error("window URL required");
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));

// Runs in the page before the program's own scripts.
const bridge = `
  if (!localStorage.getItem("cargo-dispatch-planner-locale"))
    localStorage.setItem("cargo-dispatch-planner-locale", "uk");
  window.calls = [];
  window.pending = 0;
  window.__TAURI__ = { core: { invoke: async (command, args) => {
    window.pending++;
    window.calls.push(command);
    try {
      const response = await fetch("/invoke", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ command, args: args ?? {} }),
      });
      const body = await response.json();
      if (!response.ok) throw body.error;
      return body.value;
    } finally {
      window.pending--;
    }
  } } };
  const ownText = node => [...node.childNodes]
    .filter(child => child.nodeType === 3).map(child => child.textContent).join("").trim();
  window.ui = {
    section: title => [...document.querySelectorAll("section")]
      .find(part => part.querySelector(":scope > h2")?.textContent === title),
    field: (scope, name) => {
      const label = [...scope.querySelectorAll("label")].find(item => ownText(item) === name);
      if (!label) throw new Error("no field " + name);
      return label.querySelector("input, select");
    },
    button: (scope, name) => [...scope.querySelectorAll("button")]
      .find(item => item.textContent.trim() === name),
    orders: () => [...(ui.section("Заявки та варіанти маршруту")?.querySelectorAll(".cargo-card") ?? [])]
      .filter(card => card.querySelector('input[type="checkbox"]')),
    variants: () => [...(ui.section("Заявки та варіанти маршруту")?.querySelectorAll(".cargo-card") ?? [])]
      .filter(card => card.querySelector("ol")),
    orderPart: () => ui.section("Заявки та варіанти маршруту"),
    packagePart: () => ui.section("Пакет рейсу"),
    routingPart: () => ui.section("Valhalla — дорожній preview"),
    rows: () => [...document.querySelectorAll(".package-cargo")],
    // Everything the operator sees in the package section and the vehicle parameters.
    shown: () => JSON.stringify({
      vehicle: ui.field(ui.packagePart(), "Тип / клас транспорту для планування").value,
      rows: ui.rows().map(row => [...row.querySelectorAll("input, select")].map(item => item.value)),
      profile: [...ui.routingPart().querySelectorAll(".routing-grid")[1]
        .querySelectorAll("input, select")].map(item => item.value),
    }),
  };
`;

const profileFolder = mkdtempSync(join(tmpdir(), "planner-window-"));
const chrome = spawn(process.env.CHROME || "google-chrome", [
  "--headless=new", "--no-sandbox", "--no-first-run", "--disable-gpu",
  "--remote-debugging-port=0", `--user-data-dir=${profileFolder}`, "about:blank",
], { stdio: "ignore" });

let socket;
let failed = false;
let evaluateForReport;
try {
  const portFile = join(profileFolder, "DevToolsActivePort");
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
    if (message.method === "Runtime.consoleAPICalled" && message.params.type === "error") {
      exceptions.push(JSON.stringify(message.params.args));
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
  evaluateForReport = evaluate;
  const waitFor = async (what, expression, timeout = 15000) => {
    const end = Date.now() + timeout;
    while (Date.now() < end) {
      if (await evaluate(`(() => { try { return !!(${expression}); } catch { return false; } })()`)) return;
      await sleep(100);
    }
    throw new Error(`timed out waiting for: ${what}`);
  };
  // Everything the window asked of the service has been answered and drawn.
  const idle = async () => {
    await sleep(150);
    await waitFor("the service to answer", "window.pending === 0");
    await sleep(150);
  };
  const expect = async (expression, what) => {
    if (!(await evaluate(expression))) throw new Error(`failed: ${what}`);
    console.log(`PASS ${what}`);
  };
  const service = (command, args = {}) =>
    evaluate(`window.__TAURI__.core.invoke(${JSON.stringify(command)}, ${JSON.stringify(args)})`);
  const calls = command => evaluate(`window.calls.filter(item => item === "${command}").length`);
  const key = async (name, code) => {
    await send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: name, code: name, windowsVirtualKeyCode: code });
    await send("Input.dispatchKeyEvent", { type: "keyUp", key: name, code: name, windowsVirtualKeyCode: code });
  };
  // Types into a field one key at a time, replacing what it held.
  const type = async (field, text) => {
    await evaluate(`{ const item = ${field}; item.focus(); item.select(); } true`);
    for (const character of text) {
      await send("Input.dispatchKeyEvent", { type: "keyDown", key: character, text: character });
      await send("Input.dispatchKeyEvent", { type: "keyUp", key: character });
    }
  };
  const choose = (field, value) => evaluate(`{
    const item = ${field};
    item.value = "${value}";
    item.dispatchEvent(new Event("change", { bubbles: true }));
  } true`);
  const click = expression => evaluate(`${expression}.click(); true`);

  await send("Runtime.enable");
  await send("Page.enable");
  await send("Network.enable");
  await send("Emulation.setFocusEmulationEnabled", { enabled: true });
  // The map tiles belong to a public service and are not part of this test.
  await send("Network.setBlockedURLs", { urls: ["*tile.openstreetmap.org*"] });
  await send("Page.addScriptToEvaluateOnNewDocument", { source: bridge });
  await send("Page.navigate", { url: windowUrl });

  await waitFor("the orders section", "ui.orderPart() && ui.packagePart()");
  await idle();
  await click('ui.button(ui.orderPart(), "Відкрити файл заявок")');
  await waitFor("two orders", "ui.orders().length === 2");
  await idle();
  await expect('ui.orders()[0].textContent.includes("S1") && ui.orders()[1].textContent.includes("S2")',
    "the order file is opened and both orders are shown");

  await type('ui.field(ui.orders()[0], "Маса, кг")', "2400");
  await key("Tab", 9);
  await idle();
  await expect('document.activeElement === ui.field(ui.orders()[0], "Палети")',
    "after a correction the cursor stays in the next field");
  let state = await service("planner_planning_status");
  if (state.input.orders[0].cargo.weight_kg !== 2400) throw new Error("failed: the corrected weight is kept");
  console.log("PASS the corrected weight is kept");

  await click('ui.button(ui.orderPart(), "Додати заявку")');
  await waitFor("a third order", "ui.orders().length === 3");
  await idle();
  await type('ui.field(ui.orders()[2], "Вантаж")', "Вантаж вручну");
  await expect('ui.field(ui.orders()[2], "Вантаж").value === "Вантаж вручну"', "text can be typed into a new order");
  await key("Tab", 9);
  await idle();
  state = await service("planner_planning_status");
  if (state.input.orders[2]?.id !== "M3" || state.input.orders[2].cargo.title !== "Вантаж вручну")
    throw new Error("failed: the order added by hand is kept");
  console.log("PASS an order added by hand is kept");
  await click('ui.button(ui.orders()[2], "Прибрати")');
  await waitFor("two orders again", "ui.orders().length === 2");
  await idle();
  if ((await service("planner_planning_status")).input.orders.length !== 2)
    throw new Error("failed: the removed order is gone");
  console.log("PASS an order can be removed");
  await click('ui.button(ui.orderPart(), "Зберегти у файл")');
  await waitFor("the saved file", 'document.body.innerText.includes("saved-orders.json")');
  await idle();
  console.log("PASS the orders are saved to a file");

  await click('ui.button(ui.orderPart(), "Запропонувати маршрути")');
  await waitFor("variants for the complete order", "ui.variants().length >= 1");
  await idle();
  await expect('ui.orderPart().textContent.includes("S2: бракує координат")',
    "an order without coordinates is named as left out");

  await type('ui.field(ui.orders()[1].querySelectorAll(".routing-grid")[1], "Широта")', "50.2");
  await key("Tab", 9);
  await idle();
  await type('ui.field(ui.orders()[1].querySelectorAll(".routing-grid")[1], "Довгота")', "30,4");
  await key("Tab", 9);
  await idle();
  await expect("ui.variants().length === 0", "a corrected order drops the outdated variants");
  await click('ui.button(ui.orderPart(), "Запропонувати маршрути")');
  await waitFor("variants for both orders",
    'ui.variants().length >= 2 && ui.variants()[0].querySelectorAll("li").length === 4');
  await idle();
  await expect('!ui.orderPart().textContent.includes("бракує координат")', "both orders are planned");

  const first = 'ui.button(ui.variants()[0], "Обрати цей маршрут")';
  await click(first);
  await waitFor("the chosen route", 'document.querySelectorAll(".route-item").length === 4 && ui.rows().length === 2');
  await idle();
  const filled = `(() => {
    const rows = ui.rows().map(row => [...row.querySelectorAll("input, select")]);
    const titles = rows.map(items => items[0].value).sort().join("|");
    return titles === "Вантаж А|Вантаж Б"
      && rows.every(items => items[1].value && items[2].value)
      && rows.every(items => items[3].value && items[4].value && items[3].value !== items[4].value)
      && ui.field(ui.packagePart(), "Тип / клас транспорту для планування").value === "тент 20 т";
  })()`;
  await expect(filled, "the chosen variant fills the visible cargo rows with their points");
  state = await service("planner_planning_status");
  await expect(`(() => {
    const shown = ui.rows().map(row => [...row.querySelectorAll("select")].map(item => item.value).join(":")).join(",");
    return shown === ${JSON.stringify(state.cargos.map(row => `${row.pickup_point_id}:${row.unload_point_id}`).join(","))};
  })()`, "the points shown are the points of the route");

  const applied = await calls("planner_planning_apply");
  const second = "ui.variants()[1].querySelector('button')";
  await click(second);
  await idle();
  await expect(`${second}.textContent.trim() === "Натисніть ще раз: поточний маршрут буде замінено"
    && window.calls.filter(item => item === "planner_planning_apply").length === ${applied}`,
    "replacing a route asks for a second press");
  await click(second);
  await waitFor("the second variant applied",
    `window.calls.filter(item => item === "planner_planning_apply").length === ${applied + 1}`);
  await idle();
  await expect(filled, "the replaced route fills the cargo rows again");

  await expect(`(() => {
    const items = [...document.querySelectorAll(".route-item")];
    return items.every((item, index) => {
      const [up, down] = item.querySelectorAll("button");
      return item.querySelector(".sequence").textContent === String(index + 1)
        && up.textContent.trim() === "Вище" && down.textContent.trim() === "Нижче"
        && up.disabled === (index === 0) && down.disabled === (index === items.length - 1);
    });
  })()`, "the route buttons are named and enabled according to the place of the point");

  const ordered = async () => (await service("planner_status")).points.map(point => point.point_id).join();
  const initial = await ordered();
  const numbered = `ui.rows().every(row => [...row.querySelectorAll("select")].every(list =>
    [...list.options].slice(1).length === 4 && [...list.options].slice(1).every((option, index) =>
      option.textContent.trim().startsWith((index + 1) + "."))))`;
  await click('document.querySelectorAll(".route-item")[0].querySelectorAll("button")[1]');
  await idle();
  if ((await ordered()) === initial) throw new Error("failed: a point moves down");
  await expect(numbered, "the point lists follow the new order of the route");
  await expect(filled, "moving a point keeps it attached to its cargo");
  await click('document.querySelectorAll(".route-item")[1].querySelectorAll("button")[0]');
  await idle();
  if ((await ordered()) !== initial) throw new Error("failed: a point moves back");

  await type('ui.field(ui.packagePart(), "Тип / клас транспорту для планування")', "тент 22 т");
  await type('ui.field(ui.rows()[0], "Назва вантажу")', "Правка вручну");
  await type('ui.field(ui.rows()[0], "Маса, кг")', "2300");
  await expect(`ui.field(ui.packagePart(), "Тип / клас транспорту для планування").value === "тент 22 т"
    && ui.field(ui.rows()[0], "Назва вантажу").value === "Правка вручну"
    && ui.field(ui.rows()[0], "Маса, кг").value === "2300"`, "manual corrections of the package are typed in");
  await click('ui.button(ui.packagePart(), "Додати вантаж")');
  await waitFor("a third row", "ui.rows().length === 3");
  await idle();
  state = await service("planner_planning_status");
  const removedRow = state.package.cargos[2].row_id;
  await click('ui.button(ui.rows()[2], "Видалити вантаж")');
  await waitFor("two rows", "ui.rows().length === 2");
  await type('ui.field(ui.routingPart(), "Висота, м")', "4,2");
  await expect('ui.field(ui.routingPart(), "Висота, м").value === "4,2"', "a decimal can be typed key by key");
  const parameters = 'ui.routingPart().querySelectorAll(".routing-grid")[1]';
  for (const [name, value] of [
    ["ADR / небезпечний вантаж", "no"], ["Вантажний режим HGV", "yes"],
    ["Виключати ґрунтові дороги", "no"], ["Надавати перевагу вантажним дорогам", "yes"],
    ["Платні дороги", "allow"],
  ]) await choose(`ui.field(${parameters}, "${name}")`, value);
  await type('ui.field(ui.routingPart(), "Адреса Valhalla")', "http://127.0.0.1:9");
  await type('ui.field(ui.routingPart(), "ID набору карт")', "synthetic");
  await click('ui.button(ui.routingPart(), "Застосувати налаштування")');
  await waitFor("routing configured", 'document.body.innerText.includes("Valhalla налаштовано.")');
  await idle();

  const before = await evaluate("ui.shown()");
  await service("test_restart");
  await send("Page.reload");
  await waitFor("the window after a restart",
    'ui.orders().length === 2 && document.querySelectorAll(".route-item").length === 4 && ui.rows().length === 2');
  await idle();
  const after = await evaluate("ui.shown()");
  if (after !== before) throw new Error(`failed: the package form comes back\n before ${before}\n after  ${after}`);
  console.log("PASS the package form comes back exactly as left");
  await expect('ui.field(ui.orders()[0], "Маса, кг").value === "2400"', "the corrected orders come back");
  await expect('ui.field(ui.routingPart(), "Адреса Valhalla").value === "http://127.0.0.1:9"',
    "the routing settings come back");

  await click('ui.button(ui.packagePart(), "Додати вантаж")');
  await waitFor("a new row", "ui.rows().length === 3");
  await idle();
  state = await service("planner_planning_status");
  if (!(state.package.cargos[2].row_id > removedRow)) throw new Error("failed: a removed row number is not reused");
  console.log("PASS a removed cargo row number is never used again");

  await click('ui.button(ui.packagePart(), "Експортувати пакет ZIP")');
  await waitFor("the exported package", 'document.querySelector(".package-result")?.textContent.includes(".zip")');
  await idle();
  console.log("PASS the trip package for Control is exported");

  const gone = await evaluate('ui.rows()[0].querySelectorAll("select")[0].value');
  const position = (await service("planner_status")).points.findIndex(point => point.point_id === gone);
  if (position < 0) throw new Error("failed: the first cargo has a pickup point on the route");
  await click(`document.querySelectorAll(".route-item")[${position}].querySelector("button.danger")`);
  await waitFor("three points", 'document.querySelectorAll(".route-item").length === 3');
  await idle();
  await expect('ui.rows()[0].querySelectorAll("select")[0].value === ""',
    "a removed point is detached from its cargo");
  state = await service("planner_planning_status");
  if (state.package.cargos[0].pickup_point_id !== "") throw new Error("failed: the detached point is saved");
  console.log("PASS the detached point is saved");
  await expect('!document.querySelector("main > p.error")', "no error message is shown");

  for (const [locale, title] of [["en", "Orders and route variants"], ["ja", "注文とルート案"]]) {
    await choose('document.querySelector(".language select")', locale);
    await waitFor(`${locale} texts`, `ui.section("${title}")`);
    console.log(`PASS the window switches to ${locale}`);
  }

  await send("Page.reload");
  await waitFor("the saved language after a restart", `ui.section("注文とルート案")`);
  await idle();
  await expect('document.querySelector(".language select").value === "ja"',
    "the language list shows the saved language");

  if (exceptions.length) throw new Error(`failed: script errors ${exceptions.join("; ")}`);
  console.log("Planner window browser test: PASS");
} catch (error) {
  failed = true;
  console.error(error);
  try {
    console.error(await evaluateForReport?.("document.body.innerText"));
    console.error(await evaluateForReport?.("ui.shown()"));
  } catch (reportError) {
    console.error(String(reportError));
  }
} finally {
  socket?.close();
  chrome.kill();
}
process.exit(failed ? 1 : 0);
