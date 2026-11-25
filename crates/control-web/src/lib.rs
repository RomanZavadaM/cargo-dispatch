#![forbid(unsafe_code)]

use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::{Path, Request, State},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE},
    },
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use cargo_dispatch_domain::{DocumentId, PointId};
use cargo_dispatch_trip_files::{
    DocumentDirection, DocumentStoreDraft, DriverLocationSample, StoredDocument, TripFolder,
};
use futures_util::{StreamExt, stream};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path as FsPath, PathBuf},
    sync::{Arc, Mutex, RwLock},
};
use tempfile::NamedTempFile;
use tokio::{
    fs::File as TokioFile,
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
    task::JoinHandle,
};

const MAX_UPLOAD_BYTES: u64 = 512 * 1024 * 1024;
const STREAM_CHUNK_BYTES: usize = 64 * 1024;

const DRIVER_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="referrer" content="strict-origin-when-cross-origin">
<title>Cargo Dispatch — Trip</title>
<style>
:root{font-family:system-ui,sans-serif;color:#202124;background:#f6f7f8}
*{box-sizing:border-box}
body{margin:0;background:#f6f7f8;color:#202124;overflow-x:hidden}
main{width:min(100%,1440px);margin:auto;padding:20px;display:grid;grid-template-columns:minmax(0,2fr) minmax(320px,1fr);grid-template-areas:"title title" "status status" "map files" "route files" "route location";gap:14px;align-items:start}
h1{margin:0}.title-row{grid-area:title;display:flex;gap:12px;align-items:center;justify-content:space-between;flex-wrap:wrap}.language-picker{display:flex;gap:8px;align-items:center}.language-picker select{width:auto;min-width:180px}
#status{grid-area:status;margin:0;padding:10px 12px;border-radius:10px;background:#eef3f8}
section{background:white;border:1px solid #ddd;border-radius:12px;padding:16px;min-width:0}
#mapSection{grid-area:map}
#routeSection{grid-area:route}
#filesSection{grid-area:files}
#locationSection{grid-area:location}
.trip-map{position:relative;height:420px;overflow:hidden;border:1px solid #c8d0d8;border-radius:10px;background:#e7ecef;touch-action:none}
.map-tiles,.map-overlays{position:absolute;inset:0;overflow:hidden}
.map-tiles img{position:absolute;width:256px;height:256px;max-width:none;user-select:none;pointer-events:none}
.map-overlays{pointer-events:none;z-index:2}
.map-marker{position:absolute;transform:translate(-50%,-50%);display:grid;place-items:center;width:30px;height:30px;border:2px solid #fff;border-radius:999px;background:#1769aa;color:#fff;font-size:12px;font-weight:700;box-shadow:0 1px 5px #0006}
.map-marker.completed{background:#68727d}
.map-location-marker{width:18px;height:18px;background:#d63b2f;border-width:3px;box-shadow:0 0 0 5px #d63b2f33}
.map-controls{position:absolute;top:10px;right:10px;z-index:4;display:flex;gap:6px}
.map-controls button{min-width:42px;background:#fff}
.map-attribution{position:absolute;right:4px;bottom:3px;z-index:4;padding:2px 5px;border-radius:4px;background:#ffffffe6;color:#345;font-size:11px;text-decoration:none}
.map-note{margin-bottom:0;color:#5f6670;font-size:13px}
button,input,select{font:inherit;max-width:100%}
button{min-height:40px;padding:8px 12px;border:1px solid #b8bec6;border-radius:8px;background:#fff;cursor:pointer}
button:disabled{cursor:default;opacity:.55}
input[type=file],select{width:100%;min-width:0}
label{display:grid;gap:6px;margin:10px 0}
.file-picker{flex:1 1 220px;margin:0}
.file-picker span{font-weight:600}
.row{display:flex;gap:8px;align-items:center;flex-wrap:wrap}
.point{padding:12px 0;border-bottom:1px solid #eee;min-width:0}
.point:last-child{border-bottom:0}
.point-header{display:flex;gap:10px;align-items:flex-start;justify-content:space-between;flex-wrap:wrap}
.point-title{font-weight:650;overflow-wrap:anywhere}
.point-actions{display:flex;gap:6px;flex-wrap:wrap}
.point-nav{margin-top:8px;color:#4f5660;overflow-wrap:anywhere}
.point-nav a{display:inline-block;margin-left:4px}
#documents ul{padding-left:20px;margin-bottom:0}
#documents li{overflow-wrap:anywhere;margin:6px 0}
small{color:#666}.error{color:#a40000}.ok{color:#176b2c}
@media (min-width:700px) and (max-width:1099px){
  main{grid-template-columns:minmax(0,1fr) minmax(0,1fr);grid-template-areas:"title title" "status status" "map map" "route route" "files location";padding:18px}
  .trip-map{height:360px}
}
@media (max-width:699px){
  main{display:block;width:100%;padding:12px}
  h1{font-size:1.55rem}
  #status{margin:10px 0}
  section{margin:10px 0;padding:13px}
  .row{align-items:stretch}
  .row>button,.row>input{width:100%}
  .point-header{display:block}
  .point-actions{margin-top:8px}
  .point-actions button{flex:1 1 72px}
  .trip-map{height:300px}
}
</style>
</head>
<body>
<main>
<div class="title-row">
<h1 id="appTitle">Cargo Dispatch — Trip</h1>
<label class="language-picker"><span id="languageLabel">Language</span>
<select id="languageSelect"></select>
</label>
</div>
<p id="status">Loading…</p>

<section id="mapSection">
<h2 id="mapTitle">Map</h2>
<div id="tripMap" class="trip-map" aria-label="Trip map">
  <div class="map-tiles" aria-hidden="true"></div>
  <div class="map-overlays" aria-hidden="true"></div>
  <div class="map-controls">
    <button id="mapZoomIn" type="button" aria-label="Zoom in">+</button>
    <button id="mapZoomOut" type="button" aria-label="Zoom out">−</button>
    <button id="mapFit" type="button">Trip</button>
  </div>
  <a class="map-attribution" href="https://www.openstreetmap.org/copyright" target="_blank" rel="noopener">© OpenStreetMap contributors</a>
</div>
<p id="mapNote" class="map-note"></p>
</section>

<section id="routeSection">
<h2 id="routeTitle">Route</h2>
<div id="points"></div>
</section>

<section id="filesSection">
<h2 id="filesTitle">Documents / photos / video</h2>
<div id="documents"></div>
<label><span id="attachPointLabel">Attach to point</span>
<select id="filePoint"><option value="">Whole trip</option></select>
</label>
<div class="row">
<label class="file-picker"><span id="fileLabel">File</span>
<input id="file" type="file" accept="image/*,video/*,application/pdf,*/*">
</label>
<label class="file-picker"><span id="cameraLabel">Photo / video</span>
<input id="camera" type="file" accept="image/*,video/*" capture="environment">
</label>
<button id="sendFile" type="button">Send to Control</button>
</div>
<p id="fileStatus"></p>
</section>

<section id="locationSection">
<h2 id="locationTitle">Location</h2>
<p id="location">Not sent yet</p>
<div class="row">
<button id="startLocation" type="button">Start location sharing</button>
<button id="stopLocation" type="button" disabled>Stop</button>
</div>
<p id="locationStatus"></p>
</section>
</main>
<script>
const LANGUAGES = [
  ["uk", "🇺🇦 Українська"],
  ["en", "🇬🇧 English"],
  ["de", "🇩🇪 Deutsch"],
  ["fr", "🇫🇷 Français"],
  ["es", "🇪🇸 Español"],
  ["ko", "🇰🇷 한국어"],
  ["ja", "🇯🇵 日本語"],
  ["zh-Hans", "🇨🇳 简体中文"]
];

const I18N = {
  uk: {
    title:"Cargo Dispatch — рейс", language:"Мова", loading:"Завантаження…", map:"Карта", tripMap:"Карта рейсу",
    zoomIn:"Збільшити карту", zoomOut:"Зменшити карту", fit:"Рейс",
    mapNote:"Показані точки рейсу та остання прийнята позиція водія. Дорожня лінія показується лише коли Control має актуальну geometry для поточного порядку точок.",
    route:"Маршрут", files:"Документи / фото / відео", attach:"Прив'язати до точки", wholeTrip:"Рейс у цілому",
    file:"Файл", camera:"Фото / відео", sendControl:"Передати в Control", location:"Геолокація", noLocation:"Ще не передавалась",
    startLocation:"Почати передачу геолокації", stop:"Зупинити", latestPosition:"Остання позиція водія",
    version:"Версія", change:"зміна", noTrip:"Немає доступу до рейсу", completed:"виконано", open:"відкрито",
    complete:"Виконано", moveUp:"Перемістити точку вище", moveDown:"Перемістити точку нижче",
    rejectChange:"Control не прийняв зміну", routeChanged:"Маршрут уже змінився або Control відхилив порядок",
    openNavigation:"Відкрити навігацію", noDocuments:"Документів ще немає", fromOperator:"Control → водій",
    fromDriver:"водій → Control", bytes:"байт", chooseMedia:"Спочатку виберіть файл або фото/відео",
    transferring:"Передача…", fileAccepted:"Файл прийнятий Control",
    transferFailed:"Передача не вдалася", retrySameId:"Повтор для того самого файла використає той самий ID.",
    geoUnsupported:"Браузер не підтримує геолокацію",
    geoHttps:"Для геолокації телефонний браузер зазвичай вимагає HTTPS. LAN HTTP можна використовувати для маршруту й файлів.",
    positionAccepted:"Позиція прийнята Control", positionRejected:"Control не прийняв позицію", geoUnavailable:"Геолокація недоступна"
  },
  en: {
    title:"Cargo Dispatch — Trip", language:"Language", loading:"Loading…", map:"Map", tripMap:"Trip map",
    zoomIn:"Zoom in", zoomOut:"Zoom out", fit:"Trip",
    mapNote:"Trip points and the latest accepted driver position are shown. The road line appears only when Control has current geometry for the current point order.",
    route:"Route", files:"Documents / photos / video", attach:"Attach to point", wholeTrip:"Whole trip",
    file:"File", camera:"Photo / video", sendControl:"Send to Control", location:"Location", noLocation:"Not sent yet",
    startLocation:"Start location sharing", stop:"Stop", latestPosition:"Latest driver position",
    version:"Version", change:"change", noTrip:"Trip is unavailable", completed:"completed", open:"open",
    complete:"Complete", moveUp:"Move point up", moveDown:"Move point down",
    rejectChange:"Control rejected the change", routeChanged:"The route changed or Control rejected the order",
    openNavigation:"Open navigation", noDocuments:"No documents yet", fromOperator:"Control → driver",
    fromDriver:"driver → Control", bytes:"bytes", chooseMedia:"Choose a file or photo/video first",
    transferring:"Sending…", fileAccepted:"File accepted by Control",
    transferFailed:"Transfer failed", retrySameId:"Retrying the same file will use the same ID.",
    geoUnsupported:"This browser does not support geolocation",
    geoHttps:"Phone browsers usually require HTTPS for geolocation. LAN HTTP can still be used for route and files.",
    positionAccepted:"Position accepted by Control", positionRejected:"Control rejected the position", geoUnavailable:"Geolocation unavailable"
  },
  de: {
    title:"Cargo Dispatch — Tour", language:"Sprache", loading:"Laden…", map:"Karte", tripMap:"Tourkarte",
    zoomIn:"Vergrößern", zoomOut:"Verkleinern", fit:"Tour", mapNote:"Tourpunkte und die letzte angenommene Fahrerposition werden angezeigt. Die Straßenlinie erscheint nur bei aktueller Geometrie.",
    route:"Route", files:"Dokumente / Fotos / Videos", attach:"An Punkt anhängen", wholeTrip:"Gesamte Tour", file:"Datei", camera:"Foto / Video",
    sendControl:"An Control senden", location:"Standort", noLocation:"Noch nicht gesendet", startLocation:"Standortübertragung starten", stop:"Stoppen",
    latestPosition:"Letzte Fahrerposition", version:"Version", change:"Änderung", noTrip:"Tour nicht verfügbar", completed:"erledigt", open:"offen",
    complete:"Erledigt", moveUp:"Punkt nach oben", moveDown:"Punkt nach unten", rejectChange:"Control hat die Änderung abgelehnt",
    routeChanged:"Die Route wurde geändert oder Control hat die Reihenfolge abgelehnt", openNavigation:"Navigation öffnen", noDocuments:"Noch keine Dokumente",
    fromOperator:"Control → Fahrer", fromDriver:"Fahrer → Control", bytes:"Bytes", chooseMedia:"Zuerst Datei oder Foto/Video wählen",
    transferring:"Übertragung…", fileAccepted:"Datei von Control angenommen", transferFailed:"Übertragung fehlgeschlagen",
    retrySameId:"Ein erneuter Versuch derselben Datei verwendet dieselbe ID.", geoUnsupported:"Browser unterstützt keine Geolokalisierung",
    geoHttps:"Für Geolokalisierung benötigt der Telefonbrowser normalerweise HTTPS. LAN-HTTP kann für Route und Dateien verwendet werden.",
    positionAccepted:"Position von Control angenommen", positionRejected:"Control hat die Position abgelehnt", geoUnavailable:"Geolokalisierung nicht verfügbar"
  },
  fr: {
    title:"Cargo Dispatch — Trajet", language:"Langue", loading:"Chargement…", map:"Carte", tripMap:"Carte du trajet",
    zoomIn:"Zoom avant", zoomOut:"Zoom arrière", fit:"Trajet", mapNote:"Les points du trajet et la dernière position acceptée sont affichés. La ligne routière apparaît seulement avec une géométrie actuelle.",
    route:"Itinéraire", files:"Documents / photos / vidéos", attach:"Associer au point", wholeTrip:"Trajet entier", file:"Fichier", camera:"Photo / vidéo",
    sendControl:"Envoyer à Control", location:"Géolocalisation", noLocation:"Pas encore transmise", startLocation:"Démarrer la géolocalisation", stop:"Arrêter",
    latestPosition:"Dernière position du conducteur", version:"Version", change:"modification", noTrip:"Trajet indisponible", completed:"terminé", open:"ouvert",
    complete:"Terminé", moveUp:"Monter le point", moveDown:"Descendre le point", rejectChange:"Control a refusé la modification",
    routeChanged:"Le trajet a changé ou Control a refusé l’ordre", openNavigation:"Ouvrir la navigation", noDocuments:"Aucun document",
    fromOperator:"Control → conducteur", fromDriver:"conducteur → Control", bytes:"octets", chooseMedia:"Choisissez d’abord un fichier ou une photo/vidéo",
    transferring:"Envoi…", fileAccepted:"Fichier accepté par Control", transferFailed:"Échec du transfert",
    retrySameId:"Un nouvel essai du même fichier utilisera le même ID.", geoUnsupported:"Ce navigateur ne prend pas en charge la géolocalisation",
    geoHttps:"La géolocalisation sur téléphone nécessite généralement HTTPS. Le LAN HTTP reste utilisable pour l’itinéraire et les fichiers.",
    positionAccepted:"Position acceptée par Control", positionRejected:"Control a refusé la position", geoUnavailable:"Géolocalisation indisponible"
  },
  es: {
    title:"Cargo Dispatch — Viaje", language:"Idioma", loading:"Cargando…", map:"Mapa", tripMap:"Mapa del viaje",
    zoomIn:"Acercar", zoomOut:"Alejar", fit:"Viaje", mapNote:"Se muestran los puntos del viaje y la última posición aceptada. La línea vial aparece solo con geometría actual.",
    route:"Ruta", files:"Documentos / fotos / vídeo", attach:"Adjuntar al punto", wholeTrip:"Viaje completo", file:"Archivo", camera:"Foto / vídeo",
    sendControl:"Enviar a Control", location:"Geolocalización", noLocation:"Aún no enviada", startLocation:"Iniciar geolocalización", stop:"Detener",
    latestPosition:"Última posición del conductor", version:"Versión", change:"cambio", noTrip:"Viaje no disponible", completed:"completado", open:"abierto",
    complete:"Completado", moveUp:"Mover punto arriba", moveDown:"Mover punto abajo", rejectChange:"Control rechazó el cambio",
    routeChanged:"La ruta cambió o Control rechazó el orden", openNavigation:"Abrir navegación", noDocuments:"Aún no hay documentos",
    fromOperator:"Control → conductor", fromDriver:"conductor → Control", bytes:"bytes", chooseMedia:"Elija primero un archivo o foto/vídeo",
    transferring:"Enviando…", fileAccepted:"Archivo aceptado por Control", transferFailed:"Falló la transferencia",
    retrySameId:"Reintentar el mismo archivo usará el mismo ID.", geoUnsupported:"El navegador no admite geolocalización",
    geoHttps:"La geolocalización en el navegador del teléfono suele requerir HTTPS. LAN HTTP puede usarse para ruta y archivos.",
    positionAccepted:"Posición aceptada por Control", positionRejected:"Control rechazó la posición", geoUnavailable:"Geolocalización no disponible"
  },
  ko: {
    title:"Cargo Dispatch — 운행", language:"언어", loading:"불러오는 중…", map:"지도", tripMap:"운행 지도", zoomIn:"확대", zoomOut:"축소", fit:"운행",
    mapNote:"운행 지점과 마지막으로 수신된 운전자 위치를 표시합니다. 현재 geometry가 있을 때만 도로선을 표시합니다.",
    route:"경로", files:"문서 / 사진 / 동영상", attach:"지점에 연결", wholeTrip:"전체 운행", file:"파일", camera:"사진 / 동영상",
    sendControl:"Control로 전송", location:"위치", noLocation:"아직 전송되지 않음", startLocation:"위치 전송 시작", stop:"중지",
    latestPosition:"마지막 운전자 위치", version:"버전", change:"변경", noTrip:"운행에 접근할 수 없음", completed:"완료", open:"열림",
    complete:"완료", moveUp:"지점 위로", moveDown:"지점 아래로", rejectChange:"Control이 변경을 거부했습니다",
    routeChanged:"경로가 변경되었거나 Control이 순서를 거부했습니다", openNavigation:"내비게이션 열기", noDocuments:"아직 문서가 없습니다",
    fromOperator:"Control → 운전자", fromDriver:"운전자 → Control", bytes:"바이트", chooseMedia:"먼저 파일 또는 사진/동영상을 선택하세요",
    transferring:"전송 중…", fileAccepted:"Control이 파일을 받았습니다", transferFailed:"전송 실패",
    retrySameId:"같은 파일을 다시 보내면 같은 ID를 사용합니다.", geoUnsupported:"브라우저가 위치 기능을 지원하지 않습니다",
    geoHttps:"휴대폰 브라우저의 위치 기능은 보통 HTTPS가 필요합니다. LAN HTTP는 경로와 파일에 사용할 수 있습니다.",
    positionAccepted:"Control이 위치를 받았습니다", positionRejected:"Control이 위치를 거부했습니다", geoUnavailable:"위치를 사용할 수 없습니다"
  },
  ja: {
    title:"Cargo Dispatch — 運行", language:"言語", loading:"読み込み中…", map:"地図", tripMap:"運行マップ", zoomIn:"拡大", zoomOut:"縮小", fit:"運行",
    mapNote:"運行地点と最後に受信したドライバー位置を表示します。現在の geometry がある場合のみ道路線を表示します。",
    route:"ルート", files:"文書 / 写真 / 動画", attach:"地点に関連付け", wholeTrip:"運行全体", file:"ファイル", camera:"写真 / 動画",
    sendControl:"Control へ送信", location:"位置情報", noLocation:"まだ送信されていません", startLocation:"位置送信を開始", stop:"停止",
    latestPosition:"最新のドライバー位置", version:"バージョン", change:"変更", noTrip:"運行にアクセスできません", completed:"完了", open:"未完了",
    complete:"完了", moveUp:"地点を上へ", moveDown:"地点を下へ", rejectChange:"Control が変更を拒否しました",
    routeChanged:"ルートが変更されたか Control が順序を拒否しました", openNavigation:"ナビを開く", noDocuments:"文書はまだありません",
    fromOperator:"Control → ドライバー", fromDriver:"ドライバー → Control", bytes:"バイト", chooseMedia:"先にファイルまたは写真/動画を選択してください",
    transferring:"送信中…", fileAccepted:"Control がファイルを受信しました", transferFailed:"送信に失敗しました",
    retrySameId:"同じファイルの再試行では同じ ID を使用します。", geoUnsupported:"ブラウザは位置情報に対応していません",
    geoHttps:"スマホブラウザの位置情報は通常 HTTPS が必要です。LAN HTTP はルートとファイルに使用できます。",
    positionAccepted:"Control が位置を受信しました", positionRejected:"Control が位置を拒否しました", geoUnavailable:"位置情報を利用できません"
  },
  "zh-Hans": {
    title:"Cargo Dispatch — 行程", language:"语言", loading:"正在加载…", map:"地图", tripMap:"行程地图", zoomIn:"放大", zoomOut:"缩小", fit:"行程",
    mapNote:"显示行程站点和最近接收的司机位置。只有存在当前 geometry 时才显示道路线。",
    route:"路线", files:"文档 / 照片 / 视频", attach:"关联到站点", wholeTrip:"整个行程", file:"文件", camera:"照片 / 视频",
    sendControl:"发送到 Control", location:"位置", noLocation:"尚未发送", startLocation:"开始发送位置", stop:"停止",
    latestPosition:"最近司机位置", version:"版本", change:"变更", noTrip:"无法访问行程", completed:"已完成", open:"未完成",
    complete:"已完成", moveUp:"上移站点", moveDown:"下移站点", rejectChange:"Control 拒绝了变更",
    routeChanged:"路线已改变或 Control 拒绝了顺序", openNavigation:"打开导航", noDocuments:"尚无文档",
    fromOperator:"Control → 司机", fromDriver:"司机 → Control", bytes:"字节", chooseMedia:"请先选择文件或照片/视频",
    transferring:"正在发送…", fileAccepted:"Control 已接收文件", transferFailed:"传输失败",
    retrySameId:"重试同一文件将使用相同 ID。", geoUnsupported:"浏览器不支持地理位置",
    geoHttps:"手机浏览器通常要求 HTTPS 才能使用地理位置。LAN HTTP 仍可用于路线和文件。",
    positionAccepted:"Control 已接收位置", positionRejected:"Control 拒绝了位置", geoUnavailable:"无法使用地理位置"
  }
};

function normalizeLanguage(value) {
  const lower = (value || "").toLowerCase();
  if (lower.startsWith("uk")) return "uk";
  if (lower.startsWith("de")) return "de";
  if (lower.startsWith("fr")) return "fr";
  if (lower.startsWith("es")) return "es";
  if (lower.startsWith("ko")) return "ko";
  if (lower.startsWith("ja")) return "ja";
  if (lower.startsWith("zh")) return "zh-Hans";
  return "en";
}

let uiLanguage = normalizeLanguage(localStorage.getItem("cargo-dispatch-driver-locale") || navigator.language);
function t(key) { return I18N[uiLanguage][key] || I18N.en[key] || key; }

function applyLanguage() {
  document.documentElement.lang = uiLanguage;
  document.title = t("title");
  document.getElementById("appTitle").textContent = t("title");
  document.getElementById("languageLabel").textContent = t("language");
  document.getElementById("mapTitle").textContent = t("map");
  document.getElementById("tripMap").setAttribute("aria-label", t("tripMap"));
  document.getElementById("mapZoomIn").setAttribute("aria-label", t("zoomIn"));
  document.getElementById("mapZoomOut").setAttribute("aria-label", t("zoomOut"));
  document.getElementById("mapFit").textContent = t("fit");
  document.getElementById("mapNote").textContent = t("mapNote");
  document.getElementById("routeTitle").textContent = t("route");
  document.getElementById("filesTitle").textContent = t("files");
  document.getElementById("attachPointLabel").textContent = t("attach");
  document.getElementById("fileLabel").textContent = t("file");
  document.getElementById("cameraLabel").textContent = t("camera");
  document.getElementById("sendFile").textContent = t("sendControl");
  document.getElementById("locationTitle").textContent = t("location");
  document.getElementById("startLocation").textContent = t("startLocation");
  document.getElementById("stopLocation").textContent = t("stop");
  if (!currentTrip) document.getElementById("status").textContent = t("loading");
  renderRoute();
  renderDocuments();
  renderLocation();
  renderTripMap();
}

const languageSelect = document.getElementById("languageSelect");
for (const [code, label] of LANGUAGES) {
  const option = document.createElement("option");
  option.value = code;
  option.textContent = label;
  languageSelect.append(option);
}
languageSelect.value = uiLanguage;
languageSelect.addEventListener("change", () => {
  uiLanguage = normalizeLanguage(languageSelect.value);
  localStorage.setItem("cargo-dispatch-driver-locale", uiLanguage);
  applyLanguage();
});

const token = decodeURIComponent(location.pathname.split("/").pop());
let currentTrip = null;
let watchId = null;
let lastLocationSentAt = 0;
let pendingUpload = null;

const tripMapState = {
  zoom: 6,
  centerLat: 49.0,
  centerLon: 31.0,
  initialized: false,
  dragging: false,
  dragStartX: 0,
  dragStartY: 0,
  dragCenterX: 0,
  dragCenterY: 0,
  dataSignature: ""
};

function mapClampLat(latitude) {
  return Math.max(-85.05112878, Math.min(85.05112878, latitude));
}

function mapWorldSize(zoom) {
  return 256 * Math.pow(2, zoom);
}

function mapProject(latitude, longitude, zoom) {
  const size = mapWorldSize(zoom);
  const lat = mapClampLat(latitude) * Math.PI / 180;
  const sin = Math.sin(lat);
  return {
    x: (longitude + 180) / 360 * size,
    y: (0.5 - Math.log((1 + sin) / (1 - sin)) / (4 * Math.PI)) * size
  };
}

function mapUnproject(x, y, zoom) {
  const size = mapWorldSize(zoom);
  const longitude = x / size * 360 - 180;
  const mercator = Math.PI * (1 - 2 * y / size);
  const latitude = Math.atan(Math.sinh(mercator)) * 180 / Math.PI;
  return {latitude: mapClampLat(latitude), longitude};
}

function tripMapMarkerCoordinates() {
  if (!currentTrip) return [];
  const coordinates = [];
  for (const point of currentTrip.points) {
    if (point.navigation) {
      coordinates.push({
        latitude: point.navigation.latitude,
        longitude: point.navigation.longitude,
        type: "point",
        sequence: point.sequence,
        title: point.title,
        completed: point.completed
      });
    }
  }
  if (currentTrip.latest_location) {
    coordinates.push({
      latitude: currentTrip.latest_location.latitude,
      longitude: currentTrip.latest_location.longitude,
      type: "location",
      title: t("latestPosition")
    });
  }
  return coordinates;
}

function tripMapExtentCoordinates() {
  const coordinates = tripMapMarkerCoordinates().map(item => ({
    latitude: item.latitude,
    longitude: item.longitude
  }));
  if (currentTrip && currentTrip.road_geometry) {
    for (const point of currentTrip.road_geometry.points) {
      coordinates.push({
        latitude: point.latitude,
        longitude: point.longitude
      });
    }
  }
  return coordinates;
}

function renderTripMap() {
  const host = document.getElementById("tripMap");
  if (!host) return;
  const width = Math.max(1, host.clientWidth);
  const height = Math.max(1, host.clientHeight);
  const zoom = tripMapState.zoom;
  const center = mapProject(tripMapState.centerLat, tripMapState.centerLon, zoom);
  const left = center.x - width / 2;
  const top = center.y - height / 2;
  const tileLayer = host.querySelector(".map-tiles");
  const overlayLayer = host.querySelector(".map-overlays");
  tileLayer.replaceChildren();
  overlayLayer.replaceChildren();

  const tileMinX = Math.floor(left / 256) - 1;
  const tileMaxX = Math.floor((left + width) / 256) + 1;
  const tileMinY = Math.floor(top / 256) - 1;
  const tileMaxY = Math.floor((top + height) / 256) + 1;
  const tileCount = Math.pow(2, zoom);
  for (let tileX = tileMinX; tileX <= tileMaxX; tileX++) {
    const wrappedX = ((tileX % tileCount) + tileCount) % tileCount;
    for (let tileY = tileMinY; tileY <= tileMaxY; tileY++) {
      if (tileY < 0 || tileY >= tileCount) continue;
      const image = document.createElement("img");
      image.alt = "";
      image.draggable = false;
      image.loading = "lazy";
      image.src = "https://tile.openstreetmap.org/" + zoom + "/" + wrappedX + "/" + tileY + ".png";
      image.style.left = (tileX * 256 - left) + "px";
      image.style.top = (tileY * 256 - top) + "px";
      image.addEventListener("error", () => image.remove());
      tileLayer.append(image);
    }
  }

  if (currentTrip && currentTrip.road_geometry && currentTrip.road_geometry.points.length >= 2) {
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("viewBox", "0 0 " + width + " " + height);
    svg.setAttribute("width", String(width));
    svg.setAttribute("height", String(height));
    svg.style.position = "absolute";
    svg.style.inset = "0";
    const polyline = document.createElementNS("http://www.w3.org/2000/svg", "polyline");
    const shownPoints = currentTrip.road_geometry.points.map(point => {
      const projected = mapProject(point.latitude, point.longitude, zoom);
      return (projected.x - left) + "," + (projected.y - top);
    });
    polyline.setAttribute("points", shownPoints.join(" "));
    polyline.setAttribute("fill", "none");
    polyline.setAttribute("stroke", "rgb(23, 105, 170)");
    polyline.setAttribute("stroke-width", "5");
    polyline.setAttribute("stroke-linecap", "round");
    polyline.setAttribute("stroke-linejoin", "round");
    svg.append(polyline);
    overlayLayer.append(svg);
  }

  for (const item of tripMapMarkerCoordinates()) {
    const projected = mapProject(item.latitude, item.longitude, zoom);
    const marker = document.createElement("div");
    marker.className = item.type === "location"
      ? "map-marker map-location-marker"
      : "map-marker" + (item.completed ? " completed" : "");
    marker.style.left = (projected.x - left) + "px";
    marker.style.top = (projected.y - top) + "px";
    marker.title = item.title;
    marker.textContent = item.type === "location" ? "" : String(item.sequence);
    overlayLayer.append(marker);
  }
}

function fitTripMap() {
  const host = document.getElementById("tripMap");
  const coordinates = tripMapExtentCoordinates();
  if (!host || !coordinates.length) {
    tripMapState.initialized = true;
    renderTripMap();
    return;
  }
  const width = Math.max(1, host.clientWidth - 72);
  const height = Math.max(1, host.clientHeight - 72);
  let chosenZoom = 3;
  let center = {latitude: coordinates[0].latitude, longitude: coordinates[0].longitude};
  for (let zoom = 18; zoom >= 3; zoom--) {
    const projected = coordinates.map(item => mapProject(item.latitude, item.longitude, zoom));
    const xs = projected.map(item => item.x);
    const ys = projected.map(item => item.y);
    const minX = Math.min(...xs);
    const maxX = Math.max(...xs);
    const minY = Math.min(...ys);
    const maxY = Math.max(...ys);
    if (maxX - minX <= width && maxY - minY <= height) {
      chosenZoom = zoom;
      center = mapUnproject((minX + maxX) / 2, (minY + maxY) / 2, zoom);
      break;
    }
  }
  tripMapState.zoom = chosenZoom;
  tripMapState.centerLat = center.latitude;
  tripMapState.centerLon = center.longitude;
  tripMapState.initialized = true;
  renderTripMap();
}

function updateTripMap() {
  const signature = JSON.stringify({
    markers: tripMapMarkerCoordinates(),
    roadGeometry: currentTrip ? currentTrip.road_geometry : null
  });
  if (tripMapState.initialized && signature === tripMapState.dataSignature) return;
  tripMapState.dataSignature = signature;
  if (!tripMapState.initialized) {
    fitTripMap();
  } else {
    renderTripMap();
  }
}

function changeTripMapZoom(delta) {
  tripMapState.zoom = Math.max(3, Math.min(18, tripMapState.zoom + delta));
  renderTripMap();
}

function endpoint(path) {
  return "/api/trip/" + encodeURIComponent(token) + path;
}

function newDocumentId() {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  if (bytes.every(value => value === 0)) bytes[0] = 1;
  return Array.from(bytes, value => value.toString(16).padStart(2, "0")).join("");
}

async function api(path, options) {
  const response = await fetch(endpoint(path), Object.assign({cache: "no-store"}, options || {}));
  if (!response.ok) throw new Error("HTTP " + response.status);
  return response;
}

async function loadTrip() {
  try {
    const response = await api("");
    currentTrip = await response.json();
    document.getElementById("status").textContent =
      t("version") + " " + currentTrip.trip_version + ", " + t("change") + " " + currentTrip.local_route_revision;
    renderRoute();
    renderDocuments();
    renderLocation();
    updateTripMap();
  } catch (error) {
    document.getElementById("status").textContent = t("noTrip");
    document.getElementById("status").className = "error";
  }
}

function renderRoute() {
  const host = document.getElementById("points");
  host.replaceChildren();
  const pointSelect = document.getElementById("filePoint");
  const selectedPoint = pointSelect.value;
  pointSelect.replaceChildren();
  const tripOption = document.createElement("option");
  tripOption.value = "";
  tripOption.textContent = t("wholeTrip");
  pointSelect.append(tripOption);
  for (const point of currentTrip.points) {
    const option = document.createElement("option");
    option.value = point.point_id;
    option.textContent = point.sequence + ". " + point.title;
    pointSelect.append(option);
  }
  if (Array.from(pointSelect.options).some(option => option.value === selectedPoint)) {
    pointSelect.value = selectedPoint;
  }

  const openPoints = currentTrip.points.filter(point => !point.completed);
  for (const point of currentTrip.points) {
    const row = document.createElement("div");
    row.className = "point";
    const header = document.createElement("div");
    header.className = "point-header";
    const text = document.createElement("span");
    text.className = "point-title";
    text.textContent = point.sequence + ". " + point.title +
      (point.completed ? " — " + t("completed") : " — " + t("open"));
    header.append(text);

    if (!point.completed) {
      const actions = document.createElement("div");
      actions.className = "point-actions";
      const done = document.createElement("button");
      done.type = "button";
      done.textContent = t("complete");
      done.addEventListener("click", async () => {
        try {
          await api("/complete/" + point.point_id, {method: "POST"});
          await loadTrip();
        } catch (error) {
          alert(t("rejectChange") + ": " + error.message);
        }
      });
      actions.append(done);

      const openIndex = openPoints.findIndex(item => item.point_id === point.point_id);
      for (const move of [-1, 1]) {
        const button = document.createElement("button");
        button.type = "button";
        button.textContent = move < 0 ? "↑" : "↓";
        button.setAttribute("aria-label", move < 0 ? t("moveUp") : t("moveDown"));
        button.disabled =
          (move < 0 && openIndex === 0) ||
          (move > 0 && openIndex === openPoints.length - 1);
        button.addEventListener("click", async () => {
          const ids = openPoints.map(item => item.point_id);
          const target = openIndex + move;
          [ids[openIndex], ids[target]] = [ids[target], ids[openIndex]];
          try {
            await api("/route/order", {
              method: "POST",
              headers: {"Content-Type": "application/json"},
              body: JSON.stringify({
                based_on_revision: currentTrip.local_route_revision,
                open_point_ids: ids
              })
            });
            await loadTrip();
          } catch (error) {
            alert(t("routeChanged") + ": " + error.message);
            await loadTrip();
          }
        });
        actions.append(button);
      }
      header.append(actions);
    }
    row.append(header);

    if (point.navigation) {
      const nav = document.createElement("div");
      nav.className = "point-nav";
      const address = point.navigation.address ? point.navigation.address + " — " : "";
      nav.textContent = address +
        point.navigation.latitude.toFixed(6) + ", " +
        point.navigation.longitude.toFixed(6) + " ";
      const link = document.createElement("a");
      link.href = "https://www.google.com/maps/dir/?api=1&destination=" +
        encodeURIComponent(point.navigation.latitude + "," + point.navigation.longitude);
      link.target = "_blank";
      link.rel = "noopener";
      link.textContent = t("openNavigation");
      nav.append(link);
      row.append(nav);
    }
    host.append(row);
  }
}

function renderDocuments() {
  const host = document.getElementById("documents");
  host.replaceChildren();
  if (!currentTrip.documents.length) {
    host.textContent = t("noDocuments");
    return;
  }
  const list = document.createElement("ul");
  for (const item of currentTrip.documents) {
    const row = document.createElement("li");
    const link = document.createElement("a");
    link.href = endpoint("/documents/" + item.document_id);
    link.textContent = item.original_name;
    const direction = item.direction === "from_operator" ? t("fromOperator") : t("fromDriver");
    row.append(link, " — " + direction + " — " + item.size + " " + t("bytes"));
    list.append(row);
  }
  host.append(list);
}

function renderLocation() {
  const host = document.getElementById("location");
  if (!currentTrip.latest_location) {
    host.textContent = t("noLocation");
    return;
  }
  const item = currentTrip.latest_location;
  const accuracy = item.accuracy_m == null ? "" : ", ±" + item.accuracy_m.toFixed(1) + " m";
  host.textContent =
    item.latitude.toFixed(6) + ", " + item.longitude.toFixed(6) + accuracy +
    " — " + new Date(item.observed_at_unix_ms).toLocaleString();
}

document.getElementById("sendFile").addEventListener("click", async () => {
  const input = document.getElementById("file");
  const camera = document.getElementById("camera");
  const status = document.getElementById("fileStatus");
  const file =
    input.files && input.files.length ? input.files[0] :
    camera.files && camera.files.length ? camera.files[0] : null;
  if (!file) {
    status.textContent = t("chooseMedia");
    status.className = "error";
    return;
  }
  const pointId = document.getElementById("filePoint").value;
  const signature = [
    file.name,
    file.size,
    file.lastModified,
    file.type,
    pointId
  ].join("|");
  if (!pendingUpload || pendingUpload.signature !== signature) {
    pendingUpload = {signature, documentId: newDocumentId()};
  }
  const documentId = pendingUpload.documentId;
  status.textContent = t("transferring");
  status.className = "";
  try {
    const headers = {
      "Content-Type": file.type || "application/octet-stream",
      "X-File-Name": encodeURIComponent(file.name),
      "X-Created-At-Unix-Ms": String(Date.now())
    };
    if (pointId) headers["X-Point-Id"] = pointId;
    await api("/documents/" + documentId, {
      method: "POST",
      headers,
      body: file
    });
    input.value = "";
    camera.value = "";
    pendingUpload = null;
    status.textContent = t("fileAccepted");
    status.className = "ok";
    await loadTrip();
  } catch (error) {
    status.textContent =
      t("transferFailed") + ": " + error.message + ". " + t("retrySameId");
    status.className = "error";
  }
});

document.getElementById("startLocation").addEventListener("click", () => {
  const status = document.getElementById("locationStatus");
  if (!navigator.geolocation) {
    status.textContent = t("geoUnsupported");
    status.className = "error";
    return;
  }
  if (!window.isSecureContext &&
      location.hostname !== "127.0.0.1" &&
      location.hostname !== "localhost") {
    status.textContent = t("geoHttps");
    status.className = "error";
  }
  watchId = navigator.geolocation.watchPosition(async position => {
    const now = Date.now();
    if (now - lastLocationSentAt < 10000) return;
    lastLocationSentAt = now;
    try {
      await api("/location", {
        method: "POST",
        headers: {"Content-Type": "application/json"},
        body: JSON.stringify({
          latitude: position.coords.latitude,
          longitude: position.coords.longitude,
          accuracy_m: position.coords.accuracy,
          observed_at_unix_ms: Math.round(position.timestamp || now)
        })
      });
      status.textContent = t("positionAccepted");
      status.className = "ok";
      await loadTrip();
    } catch (error) {
      status.textContent = t("positionRejected") + ": " + error.message;
      status.className = "error";
    }
  }, error => {
    status.textContent = t("geoUnavailable") + ": " + error.message;
    status.className = "error";
  }, {enableHighAccuracy: true, maximumAge: 5000, timeout: 15000});
  document.getElementById("startLocation").disabled = true;
  document.getElementById("stopLocation").disabled = false;
});

document.getElementById("stopLocation").addEventListener("click", () => {
  if (watchId != null) navigator.geolocation.clearWatch(watchId);
  watchId = null;
  document.getElementById("startLocation").disabled = false;
  document.getElementById("stopLocation").disabled = true;
});

document.getElementById("mapZoomIn").addEventListener("click", () => changeTripMapZoom(1));
document.getElementById("mapZoomOut").addEventListener("click", () => changeTripMapZoom(-1));
document.getElementById("mapFit").addEventListener("click", fitTripMap);
const tripMapHost = document.getElementById("tripMap");
tripMapHost.addEventListener("wheel", event => {
  event.preventDefault();
  changeTripMapZoom(event.deltaY < 0 ? 1 : -1);
}, {passive: false});
tripMapHost.addEventListener("pointerdown", event => {
  if (event.target.closest("button,a")) return;
  const center = mapProject(tripMapState.centerLat, tripMapState.centerLon, tripMapState.zoom);
  tripMapState.dragging = true;
  tripMapState.dragStartX = event.clientX;
  tripMapState.dragStartY = event.clientY;
  tripMapState.dragCenterX = center.x;
  tripMapState.dragCenterY = center.y;
  tripMapHost.setPointerCapture(event.pointerId);
});
tripMapHost.addEventListener("pointermove", event => {
  if (!tripMapState.dragging) return;
  const x = tripMapState.dragCenterX - (event.clientX - tripMapState.dragStartX);
  const y = tripMapState.dragCenterY - (event.clientY - tripMapState.dragStartY);
  const center = mapUnproject(x, y, tripMapState.zoom);
  tripMapState.centerLat = center.latitude;
  tripMapState.centerLon = center.longitude;
  renderTripMap();
});
tripMapHost.addEventListener("pointerup", event => {
  tripMapState.dragging = false;
  if (tripMapHost.hasPointerCapture(event.pointerId)) tripMapHost.releasePointerCapture(event.pointerId);
});
tripMapHost.addEventListener("pointercancel", () => {
  tripMapState.dragging = false;
});
window.addEventListener("resize", renderTripMap);

applyLanguage();
loadTrip();
setInterval(loadTrip, 2000);
</script>
</body>
</html>
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionRegistrationError {
    InvalidToken,
    DuplicateToken,
    StateUnavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DriverNavigationSnapshot {
    pub latitude: f64,
    pub longitude: f64,
    pub address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DriverPointSnapshot {
    pub point_id: String,
    pub sequence: u32,
    pub title: String,
    pub completed: bool,
    pub navigation: Option<DriverNavigationSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DriverLocationSnapshot {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy_m: Option<f64>,
    pub observed_at_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DriverDocumentSnapshot {
    pub document_id: String,
    pub direction: String,
    pub original_name: String,
    pub mime_type: String,
    pub size: u64,
    pub point_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DriverRoadGeometryPointSnapshot {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DriverRoutingIdentitySnapshot {
    pub engine: String,
    pub engine_version: String,
    pub map_dataset_id: String,
    pub profile_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DriverRoadGeometrySnapshot {
    pub routing: DriverRoutingIdentitySnapshot,
    pub points: Vec<DriverRoadGeometryPointSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DriverTripSnapshot {
    pub trip_id: String,
    pub trip_version: u64,
    pub local_route_revision: u64,
    pub points: Vec<DriverPointSnapshot>,
    pub road_geometry: Option<DriverRoadGeometrySnapshot>,
    pub documents: Vec<DriverDocumentSnapshot>,
    pub latest_location: Option<DriverLocationSnapshot>,
}

#[derive(Debug, Clone, Deserialize)]
struct RouteOrderRequest {
    based_on_revision: u64,
    open_point_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct LocationUpdate {
    latitude: f64,
    longitude: f64,
    accuracy_m: Option<f64>,
    observed_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlAccessMode {
    Loopback,
    Lan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlServerInfo {
    pub mode: ControlAccessMode,
    pub address: SocketAddr,
}

type SharedTrip = Arc<Mutex<TripFolder>>;

#[derive(Clone, Default)]
pub struct ControlBrowserState {
    sessions: Arc<RwLock<BTreeMap<String, SharedTrip>>>,
}

impl ControlBrowserState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_trip(
        &self,
        token: impl Into<String>,
        trip: TripFolder,
    ) -> Result<(), SessionRegistrationError> {
        let token = token.into();
        if !valid_token(&token) {
            return Err(SessionRegistrationError::InvalidToken);
        }
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| SessionRegistrationError::StateUnavailable)?;
        if sessions.contains_key(&token) {
            return Err(SessionRegistrationError::DuplicateToken);
        }
        sessions.insert(token, Arc::new(Mutex::new(trip)));
        Ok(())
    }

    fn trip(&self, token: &str) -> Result<Option<SharedTrip>, StatusCode> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(sessions.get(token).cloned())
    }

    fn shared_trip(&self, token: &str) -> io::Result<SharedTrip> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| io::Error::other("browser session registry unavailable"))?;
        sessions
            .get(token)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "trip session not found"))
    }

    pub fn session_snapshot(&self, token: &str) -> io::Result<DriverTripSnapshot> {
        let trip = self.shared_trip(token)?;
        let trip = trip
            .lock()
            .map_err(|_| io::Error::other("trip state unavailable"))?;
        Ok(snapshot(&trip))
    }

    pub fn reorder_session_route(
        &self,
        token: &str,
        based_on_revision: u64,
        order: &[PointId],
    ) -> io::Result<DriverTripSnapshot> {
        let trip = self.shared_trip(token)?;
        let mut trip = trip
            .lock()
            .map_err(|_| io::Error::other("trip state unavailable"))?;
        let shown = trip.state();
        if shown.local_route_revision != based_on_revision {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "stale route revision",
            ));
        }
        trip.driver_order(&shown, order)?;
        Ok(snapshot(&trip))
    }

    pub fn store_operator_document(
        &self,
        token: &str,
        mut draft: DocumentStoreDraft,
        source: &FsPath,
    ) -> io::Result<DriverTripSnapshot> {
        draft.direction = DocumentDirection::FromOperator;
        let trip = self.shared_trip(token)?;
        let mut trip = trip
            .lock()
            .map_err(|_| io::Error::other("trip state unavailable"))?;
        trip.store_document(draft, source)?;
        Ok(snapshot(&trip))
    }

    pub fn verified_session_document_path(
        &self,
        token: &str,
        document_id: DocumentId,
    ) -> io::Result<(StoredDocument, PathBuf)> {
        let trip = self.shared_trip(token)?;
        let trip = trip
            .lock()
            .map_err(|_| io::Error::other("trip state unavailable"))?;
        trip.verified_document_path(document_id)
    }
}

fn valid_token(token: &str) -> bool {
    (16..=128).contains(&token.len())
        && token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn id_hex(bytes: [u8; 16]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn parse_uuid_bytes(value: &str) -> Result<[u8; 16], StatusCode> {
    if value.len() != 32 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut bytes = [0_u8; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&value[start..start + 2], 16)
            .map_err(|_| StatusCode::BAD_REQUEST)?;
    }
    Ok(bytes)
}

fn parse_point_id(value: &str) -> Result<PointId, StatusCode> {
    PointId::from_bytes(parse_uuid_bytes(value)?).map_err(|_| StatusCode::BAD_REQUEST)
}

fn parse_document_id(value: &str) -> Result<DocumentId, StatusCode> {
    DocumentId::from_bytes(parse_uuid_bytes(value)?).map_err(|_| StatusCode::BAD_REQUEST)
}

fn location_sample(update: LocationUpdate) -> Result<DriverLocationSample, StatusCode> {
    if !update.latitude.is_finite()
        || !(-90.0..=90.0).contains(&update.latitude)
        || !update.longitude.is_finite()
        || !(-180.0..=180.0).contains(&update.longitude)
        || update.observed_at_unix_ms == 0
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    let latitude_e7 = (update.latitude * 10_000_000.0).round();
    let longitude_e7 = (update.longitude * 10_000_000.0).round();
    if latitude_e7 < f64::from(i32::MIN)
        || latitude_e7 > f64::from(i32::MAX)
        || longitude_e7 < f64::from(i32::MIN)
        || longitude_e7 > f64::from(i32::MAX)
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    let accuracy_cm = match update.accuracy_m {
        Some(value)
            if value.is_finite() && value >= 0.0 && value <= f64::from(u32::MAX) / 100.0 =>
        {
            Some((value * 100.0).round() as u32)
        }
        Some(_) => return Err(StatusCode::BAD_REQUEST),
        None => None,
    };

    DriverLocationSample {
        latitude_e7: latitude_e7 as i32,
        longitude_e7: longitude_e7 as i32,
        accuracy_cm,
        observed_at_unix_ms: update.observed_at_unix_ms,
    }
    .validate()
    .map_err(|_| StatusCode::BAD_REQUEST)
}

fn document_snapshot(document: &StoredDocument) -> DriverDocumentSnapshot {
    DriverDocumentSnapshot {
        document_id: id_hex(document.document_id.as_bytes()),
        direction: match document.direction {
            DocumentDirection::FromOperator => "from_operator",
            DocumentDirection::FromDriver => "from_driver",
        }
        .to_owned(),
        original_name: document.original_name.clone(),
        mime_type: document.mime_type.clone(),
        size: document.size,
        point_id: document.point_id.map(|point| id_hex(point.as_bytes())),
    }
}

fn location_snapshot(sample: DriverLocationSample) -> DriverLocationSnapshot {
    DriverLocationSnapshot {
        latitude: sample.latitude(),
        longitude: sample.longitude(),
        accuracy_m: sample.accuracy_m(),
        observed_at_unix_ms: sample.observed_at_unix_ms,
    }
}

fn snapshot(trip: &TripFolder) -> DriverTripSnapshot {
    let state = trip.state();
    DriverTripSnapshot {
        trip_id: id_hex(state.trip_id.as_bytes()),
        trip_version: state.current_version.get(),
        local_route_revision: state.local_route_revision,
        points: state
            .route_points
            .iter()
            .map(|point| DriverPointSnapshot {
                point_id: id_hex(point.point_id.as_bytes()),
                sequence: point.sequence,
                title: point.title.clone(),
                completed: state.completed_point_ids.contains(&point.point_id),
                navigation: point
                    .navigation
                    .as_ref()
                    .map(|navigation| DriverNavigationSnapshot {
                        latitude: navigation.latitude(),
                        longitude: navigation.longitude(),
                        address: navigation.address.clone(),
                    }),
            })
            .collect(),
        road_geometry: state
            .route_geometry
            .as_ref()
            .map(|geometry| DriverRoadGeometrySnapshot {
                routing: DriverRoutingIdentitySnapshot {
                    engine: geometry.routing.engine.clone(),
                    engine_version: geometry.routing.engine_version.clone(),
                    map_dataset_id: geometry.routing.map_dataset_id.clone(),
                    profile_fingerprint: geometry.routing.profile_fingerprint.clone(),
                },
                points: geometry
                    .points
                    .iter()
                    .map(|point| DriverRoadGeometryPointSnapshot {
                        latitude: point.latitude(),
                        longitude: point.longitude(),
                    })
                    .collect(),
            }),
        documents: trip.documents().iter().map(document_snapshot).collect(),
        latest_location: trip.latest_location().map(location_snapshot),
    }
}

fn decode_uri_component(value: &str) -> Result<String, StatusCode> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err(StatusCode::BAD_REQUEST);
            }
            let high = (bytes[index + 1] as char)
                .to_digit(16)
                .ok_or(StatusCode::BAD_REQUEST)?;
            let low = (bytes[index + 2] as char)
                .to_digit(16)
                .ok_or(StatusCode::BAD_REQUEST)?;
            decoded.push(((high << 4) | low) as u8);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).map_err(|_| StatusCode::BAD_REQUEST)
}

fn validate_original_name(value: &str) -> Result<(), StatusCode> {
    if value.trim().is_empty()
        || value.len() > 512
        || value.contains('/')
        || value.contains('\\')
        || value.contains('\0')
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(())
}

fn encode_header_filename(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(*byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn header_text(headers: &HeaderMap, name: &'static str) -> Result<String, StatusCode> {
    headers
        .get(name)
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_str()
        .map(str::to_owned)
        .map_err(|_| StatusCode::BAD_REQUEST)
}

async fn driver_page(
    State(state): State<ControlBrowserState>,
    Path(token): Path<String>,
) -> Response {
    match state.trip(&token) {
        Ok(Some(_)) => Html(DRIVER_HTML).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "trip session not found").into_response(),
        Err(status) => status.into_response(),
    }
}

async fn trip_snapshot(
    State(state): State<ControlBrowserState>,
    Path(token): Path<String>,
) -> Response {
    let trip = match state.trip(&token) {
        Ok(Some(trip)) => trip,
        Ok(None) => return (StatusCode::NOT_FOUND, "trip session not found").into_response(),
        Err(status) => return status.into_response(),
    };
    let trip = match trip.lock() {
        Ok(trip) => trip,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    Json(snapshot(&trip)).into_response()
}

async fn complete_point(
    State(state): State<ControlBrowserState>,
    Path((token, point_id)): Path<(String, String)>,
) -> Response {
    let point_id = match parse_point_id(&point_id) {
        Ok(point_id) => point_id,
        Err(status) => return status.into_response(),
    };
    let trip = match state.trip(&token) {
        Ok(Some(trip)) => trip,
        Ok(None) => return (StatusCode::NOT_FOUND, "trip session not found").into_response(),
        Err(status) => return status.into_response(),
    };
    let mut trip = match trip.lock() {
        Ok(trip) => trip,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    if trip.complete(point_id).is_err() {
        return (StatusCode::CONFLICT, "point cannot be completed").into_response();
    }
    Json(snapshot(&trip)).into_response()
}

async fn reorder_route(
    State(state): State<ControlBrowserState>,
    Path(token): Path<String>,
    Json(request): Json<RouteOrderRequest>,
) -> Response {
    let trip = match state.trip(&token) {
        Ok(Some(trip)) => trip,
        Ok(None) => return (StatusCode::NOT_FOUND, "trip session not found").into_response(),
        Err(status) => return status.into_response(),
    };
    let mut trip = match trip.lock() {
        Ok(trip) => trip,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let shown = trip.state();
    if shown.local_route_revision != request.based_on_revision {
        return (StatusCode::CONFLICT, "stale route revision").into_response();
    }
    let order = match request
        .open_point_ids
        .iter()
        .map(|value| parse_point_id(value))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(order) => order,
        Err(status) => return status.into_response(),
    };
    if trip.driver_order(&shown, &order).is_err() {
        return (StatusCode::CONFLICT, "route order rejected").into_response();
    }
    Json(snapshot(&trip)).into_response()
}

async fn update_location(
    State(state): State<ControlBrowserState>,
    Path(token): Path<String>,
    Json(update): Json<LocationUpdate>,
) -> Response {
    let sample = match location_sample(update) {
        Ok(sample) => sample,
        Err(status) => return status.into_response(),
    };
    let trip = match state.trip(&token) {
        Ok(Some(trip)) => trip,
        Ok(None) => return (StatusCode::NOT_FOUND, "trip session not found").into_response(),
        Err(status) => return status.into_response(),
    };
    let mut trip = match trip.lock() {
        Ok(trip) => trip,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    if trip.record_location(sample).is_err() {
        return (StatusCode::CONFLICT, "location not stored").into_response();
    }
    Json(location_snapshot(sample)).into_response()
}

async fn upload_document(
    State(state): State<ControlBrowserState>,
    Path((token, document_id)): Path<(String, String)>,
    request: Request,
) -> Response {
    let document_id = match parse_document_id(&document_id) {
        Ok(document_id) => document_id,
        Err(status) => return status.into_response(),
    };
    let trip = match state.trip(&token) {
        Ok(Some(trip)) => trip,
        Ok(None) => return (StatusCode::NOT_FOUND, "trip session not found").into_response(),
        Err(status) => return status.into_response(),
    };

    let headers = request.headers();
    if let Some(length) = headers
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        && length > MAX_UPLOAD_BYTES
    {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    }

    let original_name = match header_text(headers, "x-file-name")
        .and_then(|value| decode_uri_component(&value))
        .and_then(|value| {
            validate_original_name(&value)?;
            Ok(value)
        }) {
        Ok(value) => value,
        Err(status) => return status.into_response(),
    };
    let mime_type = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty() && value.len() <= 256)
        .unwrap_or("application/octet-stream")
        .to_owned();
    let created_at_unix_ms = match header_text(headers, "x-created-at-unix-ms")
        .and_then(|value| value.parse::<u64>().map_err(|_| StatusCode::BAD_REQUEST))
    {
        Ok(value) if value > 0 => value,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    let point_id = match headers.get("x-point-id") {
        Some(value) => match value
            .to_str()
            .map_err(|_| StatusCode::BAD_REQUEST)
            .and_then(parse_point_id)
        {
            Ok(value) => Some(value),
            Err(status) => return status.into_response(),
        },
        None => None,
    };

    let temporary = match NamedTempFile::new() {
        Ok(file) => file,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let std_file = match temporary.reopen() {
        Ok(file) => file,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let mut file = TokioFile::from_std(std_file);
    let mut stream = request.into_body().into_data_stream();
    let mut total = 0_u64;
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        };
        total = match total.checked_add(u64::try_from(chunk.len()).unwrap_or(u64::MAX)) {
            Some(total) if total <= MAX_UPLOAD_BYTES => total,
            _ => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
        };
        if file.write_all(&chunk).await.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
    if file.sync_all().await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    drop(file);

    let draft = DocumentStoreDraft {
        document_id,
        direction: DocumentDirection::FromDriver,
        original_name,
        mime_type,
        created_at_unix_ms,
        point_id,
    };
    let stored = match tokio::task::spawn_blocking(move || {
        let _keep_temporary_alive = &temporary;
        let mut trip = trip
            .lock()
            .map_err(|_| io::Error::other("trip state unavailable"))?;
        trip.store_document(draft, temporary.path())
    })
    .await
    {
        Ok(Ok(stored)) => stored,
        Ok(Err(error)) if error.kind() == io::ErrorKind::InvalidData => {
            return StatusCode::CONFLICT.into_response();
        }
        _ => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    Json(document_snapshot(&stored)).into_response()
}

fn file_body(file: TokioFile) -> Body {
    let stream = stream::unfold(Some(file), |state| async move {
        let mut file = state?;
        let mut buffer = vec![0_u8; STREAM_CHUNK_BYTES];
        match file.read(&mut buffer).await {
            Ok(0) => None,
            Ok(read) => {
                buffer.truncate(read);
                Some((Ok::<Bytes, io::Error>(Bytes::from(buffer)), Some(file)))
            }
            Err(error) => Some((Err(error), None)),
        }
    });
    Body::from_stream(stream)
}

async fn download_document(
    State(state): State<ControlBrowserState>,
    Path((token, document_id)): Path<(String, String)>,
) -> Response {
    let document_id = match parse_document_id(&document_id) {
        Ok(document_id) => document_id,
        Err(status) => return status.into_response(),
    };
    let trip = match state.trip(&token) {
        Ok(Some(trip)) => trip,
        Ok(None) => return (StatusCode::NOT_FOUND, "trip session not found").into_response(),
        Err(status) => return status.into_response(),
    };

    let verified = match tokio::task::spawn_blocking(move || {
        let trip = trip
            .lock()
            .map_err(|_| io::Error::other("trip state unavailable"))?;
        trip.verified_document_path(document_id)
    })
    .await
    {
        Ok(Ok(value)) => value,
        Ok(Err(error)) if error.kind() == io::ErrorKind::NotFound => {
            return StatusCode::NOT_FOUND.into_response();
        }
        Ok(Err(_)) => return StatusCode::CONFLICT.into_response(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let (document, path) = verified;
    let file = match TokioFile::open(path).await {
        Ok(file) => file,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    let mut response = Response::new(file_body(file));
    *response.status_mut() = StatusCode::OK;
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(&document.mime_type) {
        headers.insert(CONTENT_TYPE, value);
    } else {
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/octet-stream"),
        );
    }
    if let Ok(value) = HeaderValue::from_str(&document.size.to_string()) {
        headers.insert(CONTENT_LENGTH, value);
    }
    let disposition = format!(
        "attachment; filename*=UTF-8''{}",
        encode_header_filename(&document.original_name)
    );
    if let Ok(value) = HeaderValue::from_str(&disposition) {
        headers.insert(CONTENT_DISPOSITION, value);
    }
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn router(state: ControlBrowserState) -> Router {
    Router::new()
        .route("/driver/{token}", get(driver_page))
        .route("/api/trip/{token}", get(trip_snapshot))
        .route(
            "/api/trip/{token}/complete/{point_id}",
            post(complete_point),
        )
        .route("/api/trip/{token}/route/order", post(reorder_route))
        .route("/api/trip/{token}/location", post(update_location))
        .route(
            "/api/trip/{token}/documents/{document_id}",
            get(download_document).post(upload_document),
        )
        .with_state(state)
}

fn is_private_lan_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_private() && !ip.is_loopback() && !ip.is_unspecified(),
        IpAddr::V6(ip) => {
            let first = ip.segments()[0];
            let unique_local = first & 0xfe00 == 0xfc00;
            let link_local = first & 0xffc0 == 0xfe80;
            (unique_local || link_local) && !ip.is_loopback() && !ip.is_unspecified()
        }
    }
}

fn validate_lan_address(address: SocketAddr) -> io::Result<()> {
    if address.port() == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "LAN test port must be explicit",
        ));
    }
    if !is_private_lan_ip(address.ip()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "LAN test address must be an explicit private/local interface",
        ));
    }
    Ok(())
}

pub struct LoopbackControlServer {
    info: ControlServerInfo,
    shutdown: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<io::Result<()>>>,
}

impl LoopbackControlServer {
    async fn start_bound(
        state: ControlBrowserState,
        requested: SocketAddr,
        mode: ControlAccessMode,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(requested).await?;
        let address = listener.local_addr()?;
        match mode {
            ControlAccessMode::Loopback if !address.ip().is_loopback() => {
                return Err(io::Error::other(
                    "loopback server bound outside loopback interface",
                ));
            }
            ControlAccessMode::Lan => validate_lan_address(address)?,
            _ => {}
        }

        let (shutdown, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, router(state))
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
        });

        Ok(Self {
            info: ControlServerInfo { mode, address },
            shutdown: Some(shutdown),
            task: Some(task),
        })
    }

    pub async fn start(state: ControlBrowserState) -> io::Result<Self> {
        Self::start_bound(
            state,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
            ControlAccessMode::Loopback,
        )
        .await
    }

    pub async fn start_lan(state: ControlBrowserState, address: SocketAddr) -> io::Result<Self> {
        validate_lan_address(address)?;
        Self::start_bound(state, address, ControlAccessMode::Lan).await
    }

    #[must_use]
    pub const fn info(&self) -> ControlServerInfo {
        self.info
    }

    #[must_use]
    pub const fn address(&self) -> SocketAddr {
        self.info.address
    }

    pub async fn shutdown(mut self) -> io::Result<()> {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(task) = self.task.take() {
            task.await.map_err(io::Error::other)??;
        }
        Ok(())
    }
}

impl Drop for LoopbackControlServer {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cargo_dispatch_contracts::{
        TripNavigationTarget, TripPackageHeader, TripPointKind, TripRoutePoint, TripVersion,
    };
    use cargo_dispatch_domain::{OperatorId, TripId, WorkingRouteId};
    use cargo_dispatch_planner::PlannerTripExportDraft;
    use cargo_dispatch_trip_files::{DocumentStoreDraft, TripFolder, export_package};
    use std::{fs, net::Ipv6Addr, path::Path};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn create_trip(
        root: &Path,
        name: &str,
        trip_byte: u8,
        first_point_byte: u8,
    ) -> io::Result<(TripId, Vec<TripRoutePoint>, TripFolder)> {
        let package = root.join(format!("{name}.zip"));
        let folder = root.join(name);
        let trip_id = TripId::from_bytes([trip_byte; 16]).expect("synthetic trip id");
        let route: Vec<_> = (0..3_u8)
            .map(|offset| TripRoutePoint {
                point_id: PointId::from_bytes([first_point_byte + offset; 16])
                    .expect("synthetic point id"),
                sequence: u32::from(offset) + 1,
                kind: TripPointKind::Load,
                title: format!("Synthetic point {}", offset + 1),
                navigation: Some(TripNavigationTarget {
                    latitude_e7: 498_393_000 + i32::from(offset) * 1_000,
                    longitude_e7: 240_296_000 + i32::from(offset) * 1_000,
                    address: Some(format!("Synthetic address {}", offset + 1)),
                }),
            })
            .collect();
        let draft = PlannerTripExportDraft {
            header: TripPackageHeader::new(trip_id, TripVersion::new(1).expect("version"), None)
                .expect("header"),
            working_route_id: WorkingRouteId::from_bytes([trip_byte + 1; 16]).expect("route id"),
            working_route_revision: 1,
            created_at_unix_ms: 1,
            created_by: OperatorId::from_bytes([trip_byte + 2; 16]).expect("operator"),
            ordered_point_ids: route.iter().map(|point| point.point_id).collect(),
            route_geometry: None,
        };
        export_package(&package, &draft, &route)?;
        let folder = TripFolder::import(&folder, &package)?;
        Ok((trip_id, route, folder))
    }

    fn response_parts(response: Vec<u8>) -> io::Result<(u16, String, Vec<u8>)> {
        let split = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid HTTP response"))?;
        let headers = String::from_utf8(response[..split].to_vec())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let status = headers
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|value| value.parse::<u16>().ok())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing HTTP status"))?;
        Ok((status, headers, response[split + 4..].to_vec()))
    }

    async fn http_request(
        address: SocketAddr,
        method: &str,
        path: &str,
        headers: &[(&str, String)],
        body: &[u8],
    ) -> io::Result<(u16, String, Vec<u8>)> {
        let mut stream = tokio::net::TcpStream::connect(address).await?;
        let has_content_length = headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("content-length"));
        let mut request =
            format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n");
        if !has_content_length {
            request.push_str(&format!("Content-Length: {}\r\n", body.len()));
        }
        for (name, value) in headers {
            request.push_str(name);
            request.push_str(": ");
            request.push_str(value);
            request.push_str("\r\n");
        }
        request.push_str("\r\n");
        stream.write_all(request.as_bytes()).await?;
        stream.write_all(body).await?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await?;
        response_parts(response)
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(
        address: SocketAddr,
        path: &str,
    ) -> io::Result<T> {
        let (status, _, body) = http_request(address, "GET", path, &[], &[]).await?;
        if status != 200 {
            return Err(io::Error::other(format!("unexpected HTTP {status}")));
        }
        serde_json::from_slice(&body)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    #[test]
    fn driver_page_declares_phone_tablet_and_wide_layouts() {
        assert!(DRIVER_HTML.contains("id=\"mapSection\""));
        assert!(DRIVER_HTML.contains("id=\"tripMap\""));
        assert!(DRIVER_HTML.contains("tile.openstreetmap.org"));
        assert!(DRIVER_HTML.contains("© OpenStreetMap contributors"));
        assert!(DRIVER_HTML.contains("strict-origin-when-cross-origin"));
        assert!(!DRIVER_HTML.contains("referrerPolicy = \"no-referrer\""));
        assert!(DRIVER_HTML.contains("id=\"routeSection\""));
        assert!(DRIVER_HTML.contains("id=\"filesSection\""));
        assert!(DRIVER_HTML.contains("<span>Файл</span>"));
        assert!(DRIVER_HTML.contains("<span>Фото / відео</span>"));
        assert!(DRIVER_HTML.contains("id=\"locationSection\""));
        assert!(DRIVER_HTML.contains("@media (min-width:700px) and (max-width:1099px)"));
        assert!(DRIVER_HTML.contains("@media (max-width:699px)"));
        assert!(DRIVER_HTML.contains("grid-template-columns:minmax(0,2fr) minmax(320px,1fr)"));
        assert!(!DRIVER_HTML.contains("max-width:760px"));
        assert!(DRIVER_HTML.contains("/route/order"));
        assert!(DRIVER_HTML.contains("/documents/"));
        assert!(DRIVER_HTML.contains("/location"));
    }

    #[test]
    fn lan_mode_requires_explicit_private_address_and_port() {
        assert!(validate_lan_address("192.168.1.25:8080".parse().unwrap()).is_ok());
        assert!(validate_lan_address("10.10.1.5:9090".parse().unwrap()).is_ok());
        assert!(validate_lan_address("[fd00::10]:8080".parse().unwrap()).is_ok());
        assert!(validate_lan_address("0.0.0.0:8080".parse().unwrap()).is_err());
        assert!(validate_lan_address("127.0.0.1:8080".parse().unwrap()).is_err());
        assert!(validate_lan_address("8.8.8.8:8080".parse().unwrap()).is_err());
        assert!(validate_lan_address("192.168.1.25:0".parse().unwrap()).is_err());
        assert!(!is_private_lan_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)));
    }

    #[tokio::test]
    async fn browser_exchanges_route_location_and_binary_files_with_control() -> io::Result<()> {
        let tmp = tempfile::tempdir()?;
        let folder_a_path = tmp.path().join("trip-a");
        let folder_b_path = tmp.path().join("trip-b");
        let (trip_a, route_a, mut folder_a) = create_trip(tmp.path(), "trip-a", 10, 20)?;
        let (trip_b, route_b, folder_b) = create_trip(tmp.path(), "trip-b", 30, 40)?;

        let operator_file = tmp.path().join("operator-ttn.pdf");
        fs::write(&operator_file, b"synthetic operator pdf")?;
        let operator_document = DocumentId::from_bytes([80; 16]).unwrap();
        folder_a.store_document(
            DocumentStoreDraft {
                document_id: operator_document,
                direction: DocumentDirection::FromOperator,
                original_name: "TTN-test.pdf".to_owned(),
                mime_type: "application/pdf".to_owned(),
                created_at_unix_ms: 100,
                point_id: Some(route_a[0].point_id),
            },
            &operator_file,
        )?;

        let state = ControlBrowserState::new();
        state
            .register_trip("trip-a-token-0001", folder_a)
            .expect("register trip A");
        state
            .register_trip("trip-b-token-0002", folder_b)
            .expect("register trip B");

        let server = LoopbackControlServer::start(state.clone()).await?;
        assert_eq!(server.info().mode, ControlAccessMode::Loopback);
        assert_eq!(server.address().ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));

        let (status, _, page) = http_request(
            server.address(),
            "GET",
            "/driver/trip-a-token-0001",
            &[],
            &[],
        )
        .await?;
        assert_eq!(status, 200);
        assert!(String::from_utf8_lossy(&page).contains("Передати в Control"));

        let mut trip: DriverTripSnapshot =
            get_json(server.address(), "/api/trip/trip-a-token-0001").await?;
        assert_eq!(trip.documents.len(), 1);
        assert!(trip.latest_location.is_none());

        let order_body = serde_json::to_vec(&serde_json::json!({
            "based_on_revision": trip.local_route_revision,
            "open_point_ids": [
                id_hex(route_a[2].point_id.as_bytes()),
                id_hex(route_a[0].point_id.as_bytes()),
                id_hex(route_a[1].point_id.as_bytes())
            ]
        }))?;
        let (status, _, _) = http_request(
            server.address(),
            "POST",
            "/api/trip/trip-a-token-0001/route/order",
            &[("Content-Type", "application/json".to_owned())],
            &order_body,
        )
        .await?;
        assert_eq!(status, 200);

        trip = get_json(server.address(), "/api/trip/trip-a-token-0001").await?;
        assert_eq!(
            trip.points[0].point_id,
            id_hex(route_a[2].point_id.as_bytes())
        );
        let navigation = trip.points[0]
            .navigation
            .as_ref()
            .expect("navigation target delivered");
        assert_eq!(navigation.address.as_deref(), Some("Synthetic address 3"));
        assert!((navigation.latitude - 49.8395).abs() < 0.000001);
        assert!((navigation.longitude - 24.0298).abs() < 0.000001);

        {
            let shared = state
                .trip("trip-a-token-0001")
                .map_err(|status| io::Error::other(format!("state error {status}")))?
                .expect("trip session");
            let mut control = shared
                .lock()
                .map_err(|_| io::Error::other("trip lock poisoned"))?;
            let shown = control.state();
            let mut operator_route = shown.route_points.clone();
            operator_route[0].title = "Operator updated point".to_owned();
            control.edit(&shown, &operator_route)?;
        }
        trip = get_json(server.address(), "/api/trip/trip-a-token-0001").await?;
        assert_eq!(trip.points[0].title, "Operator updated point");

        let location_body = serde_json::to_vec(&serde_json::json!({
            "latitude": 49.8393,
            "longitude": 24.0296,
            "accuracy_m": 8.5,
            "observed_at_unix_ms": 1_795_550_000_000_u64
        }))?;
        let (status, _, _) = http_request(
            server.address(),
            "POST",
            "/api/trip/trip-a-token-0001/location",
            &[("Content-Type", "application/json".to_owned())],
            &location_body,
        )
        .await?;
        assert_eq!(status, 200);

        let driver_document = DocumentId::from_bytes([81; 16]).unwrap();
        let driver_document_hex = id_hex(driver_document.as_bytes());
        let video_bytes = b"synthetic-driver-video-bytes";
        let upload_path = format!("/api/trip/trip-a-token-0001/documents/{driver_document_hex}");
        let upload_headers = [
            ("Content-Type", "video/mp4".to_owned()),
            ("X-File-Name", "driver-video.mp4".to_owned()),
            ("X-Created-At-Unix-Ms", "1795550000100".to_owned()),
            ("X-Point-Id", id_hex(route_a[2].point_id.as_bytes())),
        ];
        let (status, _, _) = http_request(
            server.address(),
            "POST",
            &upload_path,
            &upload_headers,
            video_bytes,
        )
        .await?;
        assert_eq!(status, 200);

        let (status, _, _) = http_request(
            server.address(),
            "POST",
            &upload_path,
            &upload_headers,
            video_bytes,
        )
        .await?;
        assert_eq!(status, 200);

        let different = b"different bytes for same document id";
        let (status, _, _) = http_request(
            server.address(),
            "POST",
            &upload_path,
            &upload_headers,
            different,
        )
        .await?;
        assert_eq!(status, 409);

        let photo_document = DocumentId::from_bytes([82; 16]).unwrap();
        let photo_document_hex = id_hex(photo_document.as_bytes());
        let photo_path = format!("/api/trip/trip-a-token-0001/documents/{photo_document_hex}");
        let photo_bytes = b"synthetic-driver-photo-bytes";
        let photo_headers = [
            ("Content-Type", "image/jpeg".to_owned()),
            ("X-File-Name", "driver-photo.jpg".to_owned()),
            ("X-Created-At-Unix-Ms", "1795550000200".to_owned()),
            ("X-Point-Id", id_hex(route_a[0].point_id.as_bytes())),
        ];
        let (status, _, _) = http_request(
            server.address(),
            "POST",
            &photo_path,
            &photo_headers,
            photo_bytes,
        )
        .await?;
        assert_eq!(status, 200);

        trip = get_json(server.address(), "/api/trip/trip-a-token-0001").await?;
        assert_eq!(trip.documents.len(), 3);
        let video = trip
            .documents
            .iter()
            .find(|item| item.document_id == driver_document_hex)
            .expect("driver video listed");
        assert_eq!(video.direction, "from_driver");
        assert_eq!(video.mime_type, "video/mp4");
        assert_eq!(
            video.point_id.as_deref(),
            Some(id_hex(route_a[2].point_id.as_bytes()).as_str())
        );
        let photo = trip
            .documents
            .iter()
            .find(|item| item.document_id == photo_document_hex)
            .expect("driver photo listed");
        assert_eq!(photo.mime_type, "image/jpeg");
        assert_eq!(
            photo.point_id.as_deref(),
            Some(id_hex(route_a[0].point_id.as_bytes()).as_str())
        );
        let location = trip.latest_location.expect("location stored");
        assert!((location.latitude - 49.8393).abs() < 0.000001);
        assert!((location.longitude - 24.0296).abs() < 0.000001);

        let operator_download = format!(
            "/api/trip/trip-a-token-0001/documents/{}",
            id_hex(operator_document.as_bytes())
        );
        let (status, headers, body) =
            http_request(server.address(), "GET", &operator_download, &[], &[]).await?;
        assert_eq!(status, 200);
        assert!(headers.contains("application/pdf"));
        assert_eq!(body, b"synthetic operator pdf");

        let (status, headers, body) =
            http_request(server.address(), "GET", &upload_path, &[], &[]).await?;
        assert_eq!(status, 200);
        assert!(headers.contains("video/mp4"));
        assert_eq!(body, video_bytes);

        let (status, headers, body) =
            http_request(server.address(), "GET", &photo_path, &[], &[]).await?;
        assert_eq!(status, 200);
        assert!(headers.contains("image/jpeg"));
        assert_eq!(body, photo_bytes);

        let wrong_trip_download =
            format!("/api/trip/trip-b-token-0002/documents/{driver_document_hex}");
        let (status, _, _) =
            http_request(server.address(), "GET", &wrong_trip_download, &[], &[]).await?;
        assert_eq!(status, 404);

        let trip_b_snapshot: DriverTripSnapshot =
            get_json(server.address(), "/api/trip/trip-b-token-0002").await?;
        assert!(trip_b_snapshot.documents.is_empty());
        assert!(trip_b_snapshot.latest_location.is_none());
        assert_eq!(
            trip_b_snapshot.points[0].point_id,
            id_hex(route_b[0].point_id.as_bytes())
        );

        server.shutdown().await?;
        drop(state);

        let recovered_a = TripFolder::open(&folder_a_path, trip_a)?;
        assert_eq!(
            recovered_a.state().route_points[0].point_id,
            route_a[2].point_id
        );
        assert_eq!(
            recovered_a
                .latest_location()
                .expect("durable location")
                .latitude_e7,
            498_393_000
        );
        assert_eq!(recovered_a.documents().len(), 3);
        let (_, stored_video) = recovered_a.verified_document_path(driver_document)?;
        assert_eq!(fs::read(stored_video)?, video_bytes);
        let (_, stored_photo) = recovered_a.verified_document_path(photo_document)?;
        assert_eq!(fs::read(stored_photo)?, photo_bytes);

        let recovered_b = TripFolder::open(&folder_b_path, trip_b)?;
        assert!(recovered_b.documents().is_empty());
        assert!(recovered_b.latest_location().is_none());
        assert_eq!(
            recovered_b.state().route_points[0].point_id,
            route_b[0].point_id
        );

        Ok(())
    }

    #[tokio::test]
    async fn browser_boundary_rejects_invalid_session_stale_route_and_bad_input() -> io::Result<()>
    {
        let tmp = tempfile::tempdir()?;
        let folder_path = tmp.path().join("guard-trip");
        let (trip, route, folder) = create_trip(tmp.path(), "guard-trip", 90, 100)?;

        let state = ControlBrowserState::new();
        assert_eq!(
            state.register_trip("short", folder),
            Err(SessionRegistrationError::InvalidToken)
        );
        let folder = TripFolder::open(&folder_path, trip)?;
        state
            .register_trip("guard-trip-token-0001", folder)
            .expect("valid session");

        let (_, _, duplicate) = create_trip(tmp.path(), "duplicate-trip", 110, 120)?;
        assert_eq!(
            state.register_trip("guard-trip-token-0001", duplicate),
            Err(SessionRegistrationError::DuplicateToken)
        );

        let server = LoopbackControlServer::start(state.clone()).await?;
        let (status, _, _) = http_request(
            server.address(),
            "GET",
            "/api/trip/wrong-token-0000",
            &[],
            &[],
        )
        .await?;
        assert_eq!(status, 404);

        let before: DriverTripSnapshot =
            get_json(server.address(), "/api/trip/guard-trip-token-0001").await?;
        let stale_order = serde_json::to_vec(&serde_json::json!({
            "based_on_revision": before.local_route_revision + 1,
            "open_point_ids": before
                .points
                .iter()
                .filter(|point| !point.completed)
                .map(|point| point.point_id.clone())
                .collect::<Vec<_>>()
        }))?;
        let (status, _, _) = http_request(
            server.address(),
            "POST",
            "/api/trip/guard-trip-token-0001/route/order",
            &[("Content-Type", "application/json".to_owned())],
            &stale_order,
        )
        .await?;
        assert_eq!(status, 409);

        let invalid_location = serde_json::to_vec(&serde_json::json!({
            "latitude": 95.0,
            "longitude": 24.0,
            "accuracy_m": 5.0,
            "observed_at_unix_ms": 1_u64
        }))?;
        let (status, _, _) = http_request(
            server.address(),
            "POST",
            "/api/trip/guard-trip-token-0001/location",
            &[("Content-Type", "application/json".to_owned())],
            &invalid_location,
        )
        .await?;
        assert_eq!(status, 400);

        let current_location = serde_json::to_vec(&serde_json::json!({
            "latitude": 49.8393,
            "longitude": 24.0296,
            "accuracy_m": 5.0,
            "observed_at_unix_ms": 100_u64
        }))?;
        let (status, _, _) = http_request(
            server.address(),
            "POST",
            "/api/trip/guard-trip-token-0001/location",
            &[("Content-Type", "application/json".to_owned())],
            &current_location,
        )
        .await?;
        assert_eq!(status, 200);

        let stale_location = serde_json::to_vec(&serde_json::json!({
            "latitude": 49.8400,
            "longitude": 24.0300,
            "accuracy_m": 6.0,
            "observed_at_unix_ms": 99_u64
        }))?;
        let (status, _, _) = http_request(
            server.address(),
            "POST",
            "/api/trip/guard-trip-token-0001/location",
            &[("Content-Type", "application/json".to_owned())],
            &stale_location,
        )
        .await?;
        assert_eq!(status, 409);

        let oversized_document = DocumentId::from_bytes([140; 16]).unwrap();
        let oversized_path = format!(
            "/api/trip/guard-trip-token-0001/documents/{}",
            id_hex(oversized_document.as_bytes())
        );
        let oversized_headers = [
            ("Content-Type", "video/mp4".to_owned()),
            ("X-File-Name", "too-large.mp4".to_owned()),
            ("X-Created-At-Unix-Ms", "2".to_owned()),
            ("Content-Length", (MAX_UPLOAD_BYTES + 1).to_string()),
        ];
        let (status, _, _) = http_request(
            server.address(),
            "POST",
            &oversized_path,
            &oversized_headers,
            &[],
        )
        .await?;
        assert_eq!(status, 413);

        let after: DriverTripSnapshot =
            get_json(server.address(), "/api/trip/guard-trip-token-0001").await?;
        assert_eq!(after.points, before.points);
        assert_eq!(after.local_route_revision, before.local_route_revision);
        let location = after.latest_location.expect("newest location preserved");
        assert_eq!(location.observed_at_unix_ms, 100);
        assert!((location.latitude - 49.8393).abs() < 0.000001);
        assert!((location.longitude - 24.0296).abs() < 0.000001);
        assert!(
            after
                .documents
                .iter()
                .all(|document| document.document_id != id_hex(oversized_document.as_bytes()))
        );

        server.shutdown().await?;
        drop(state);
        let recovered = TripFolder::open(&folder_path, trip)?;
        assert_eq!(
            recovered.state().route_points[0].point_id,
            route[0].point_id
        );
        assert_eq!(
            recovered
                .latest_location()
                .expect("newest durable location")
                .observed_at_unix_ms,
            100
        );
        assert!(recovered.document(oversized_document).is_none());
        Ok(())
    }
}
