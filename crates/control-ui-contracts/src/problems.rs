//! What the dispatcher reads when the Control service reports a failure.
//!
//! The service describes failures in Ukrainian, sometimes with a technical tail
//! from the operating system. This module sorts them into a few kinds and gives
//! each kind a text in the eight interface languages.

/// Interface languages, in the order of the texts below.
const LOCALES: [&str; 8] = ["uk", "en", "de", "fr", "es", "ko", "ja", "zh"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemKind {
    PortBusy,
    InvalidInput,
    SessionCode,
    AlreadyRegistered,
    TripIncomplete,
    SessionsUnavailable,
    GatewayOrder,
    TripFolder,
    TripData,
    Sharing,
    Gateway,
    Routing,
    Files,
    Window,
}

/// Fragments of service messages. The first match wins.
const FRAGMENTS: &[(&str, ProblemKind)] = &[
    ("Некоректна LAN-адреса", ProblemKind::Sharing),
    ("Некоректні налаштування Gateway", ProblemKind::Gateway),
    ("Некоректні налаштування Valhalla", ProblemKind::Routing),
    ("Session token має містити", ProblemKind::SessionCode),
    ("Цей token уже", ProblemKind::AlreadyRegistered),
    ("Цей рейс уже", ProblemKind::AlreadyRegistered),
    ("Потрібно вказати папку", ProblemKind::TripIncomplete),
    ("Додайте хоча б одну точку", ProblemKind::TripIncomplete),
    ("Кожна точка рейсу", ProblemKind::TripIncomplete),
    ("Забагато точок", ProblemKind::TripIncomplete),
    ("Список сесій", ProblemKind::SessionsUnavailable),
    ("Сесію не знайдено", ProblemKind::SessionsUnavailable),
    ("Спочатку вимкніть публічну Gateway", ProblemKind::GatewayOrder),
    ("Спочатку підключіть хоча б один", ProblemKind::GatewayOrder),
    ("Публічна Gateway-трансляція вже", ProblemKind::GatewayOrder),
    ("створити папку рейсу", ProblemKind::TripFolder),
    ("зберегти назву рейсу", ProblemKind::TripFolder),
    ("виконати створення рейсу", ProblemKind::TripFolder),
    ("відкрити задачу читання рейсу", ProblemKind::TripFolder),
    ("відкрити папку рейсу", ProblemKind::TripFolder),
    ("вибрану папку", ProblemKind::TripFolder),
    ("відкрити вибір папки", ProblemKind::TripFolder),
    ("планову основу рейсу", ProblemKind::TripData),
    ("фактичний контекст рейсу", ProblemKind::TripData),
    ("routing context", ProblemKind::TripData),
    ("прочитати рейс", ProblemKind::TripData),
    ("зберегти дані сесії", ProblemKind::TripData),
    ("створити browser session", ProblemKind::TripData),
    ("не прийняв нову черговість", ProblemKind::TripData),
    ("Browser listener", ProblemKind::Sharing),
    ("запустити loopback", ProblemKind::Sharing),
    ("запустити LAN", ProblemKind::Sharing),
    ("зупинити listener", ProblemKind::Sharing),
    ("Gateway", ProblemKind::Gateway),
    ("внутрішній Control listener", ProblemKind::Gateway),
    ("публічний Driver URL", ProblemKind::Gateway),
    ("маршрутизац", ProblemKind::Routing),
    ("Valhalla", ProblemKind::Routing),
    ("набору карт", ProblemKind::Routing),
    ("Таймаут", ProblemKind::Routing),
    ("перерахувати маршрут", ProblemKind::Routing),
    ("файл", ProblemKind::Files),
    ("Файл", ProblemKind::Files),
    ("шлях збереження", ProblemKind::Files),
    ("вікно", ProblemKind::Window),
    ("вікна", ProblemKind::Window),
    ("має містити рівно 32", ProblemKind::InvalidInput),
    ("Некоректн", ProblemKind::InvalidInput),
    ("Координата", ProblemKind::InvalidInput),
    ("Невідомий напрямок", ProblemKind::InvalidInput),
];

#[must_use]
pub fn classify_problem(message: &str) -> Option<ProblemKind> {
    // The operating system says "address already in use" with its own number.
    let lower = message.to_lowercase();
    let busy = [
        "os error 10048)",
        "os error 98)",
        "os error 48)",
        "address already in use",
    ];
    if busy.iter().any(|mark| lower.contains(mark)) {
        return Some(ProblemKind::PortBusy);
    }
    FRAGMENTS
        .iter()
        .find(|(fragment, _)| message.contains(fragment))
        .map(|(_, kind)| *kind)
}

fn texts(kind: ProblemKind) -> [&'static str; 8] {
    match kind {
        ProblemKind::PortBusy => [
            "Порт уже зайнятий іншою програмою (можливо, ще однією копією Control). Закрийте її або виберіть інший порт.",
            "The port is already used by another program (perhaps a second copy of Control). Close it or choose another port.",
            "Der Port wird bereits von einem anderen Programm benutzt (vielleicht einer zweiten Control-Instanz). Schließen Sie es oder wählen Sie einen anderen Port.",
            "Le port est déjà utilisé par un autre programme (peut-être une deuxième copie de Control). Fermez-le ou choisissez un autre port.",
            "El puerto ya lo usa otro programa (quizá una segunda copia de Control). Ciérrelo o elija otro puerto.",
            "다른 프로그램(예: Control의 두 번째 실행)이 이 포트를 사용 중입니다. 해당 프로그램을 닫거나 다른 포트를 선택하세요.",
            "このポートは別のプログラム（別の Control など）が使用中です。終了するか、別のポートを選んでください。",
            "端口已被其他程序占用（可能是另一个 Control）。请关闭它或更换端口。",
        ],
        ProblemKind::InvalidInput => [
            "Перевірте введені дані.",
            "Check the entered data.",
            "Prüfen Sie die Eingaben.",
            "Vérifiez les données saisies.",
            "Revise los datos introducidos.",
            "입력한 내용을 확인하세요.",
            "入力内容を確認してください。",
            "请检查输入的内容。",
        ],
        ProblemKind::SessionCode => [
            "Код сесії: 16–128 латинських літер, цифр, «-» або «_».",
            "Session code: 16–128 Latin letters, digits, “-” or “_”.",
            "Sitzungscode: 16–128 lateinische Buchstaben, Ziffern, „-“ oder „_“.",
            "Code de session : 16 à 128 lettres latines, chiffres, « - » ou « _ ».",
            "Código de sesión: de 16 a 128 letras latinas, cifras, «-» o «_».",
            "세션 코드: 라틴 문자, 숫자, ‘-’, ‘_’ 16~128자.",
            "セッションコード：半角英数字・「-」・「_」で16～128文字。",
            "会话代码：16–128 个拉丁字母、数字、“-”或“_”。",
        ],
        ProblemKind::AlreadyRegistered => [
            "Цей рейс або код сесії вже підключений.",
            "This trip or session code is already connected.",
            "Diese Tour oder dieser Sitzungscode ist bereits verbunden.",
            "Ce trajet ou ce code de session est déjà connecté.",
            "Este viaje o código de sesión ya está conectado.",
            "이 운행 또는 세션 코드는 이미 연결되어 있습니다.",
            "この運行またはセッションコードは接続済みです。",
            "该行程或会话代码已连接。",
        ],
        ProblemKind::TripIncomplete => [
            "Вкажіть папку рейсу, а для нового рейсу — щонайменше одну точку з назвою.",
            "Choose the trip folder and, for a new trip, at least one named stop.",
            "Wählen Sie den Tourordner und für eine neue Tour mindestens einen benannten Punkt.",
            "Choisissez le dossier du trajet et, pour un nouveau trajet, au moins un point nommé.",
            "Elija la carpeta del viaje y, para un viaje nuevo, al menos un punto con nombre.",
            "운행 폴더를 지정하고, 새 운행에는 이름 있는 지점을 하나 이상 추가하세요.",
            "運行フォルダーを指定し、新しい運行には名前付きの地点を1つ以上追加してください。",
            "请选择行程文件夹；新行程至少需要一个有名称的地点。",
        ],
        ProblemKind::SessionsUnavailable => [
            "Рейс у Control не знайдено або список рейсів тимчасово недоступний.",
            "The trip was not found in Control, or the trip list is temporarily unavailable.",
            "Die Tour wurde in Control nicht gefunden oder die Tourliste ist vorübergehend nicht verfügbar.",
            "Trajet introuvable dans Control, ou liste des trajets momentanément indisponible.",
            "El viaje no está en Control o la lista de viajes no está disponible por ahora.",
            "Control에서 운행을 찾을 수 없거나 운행 목록을 잠시 사용할 수 없습니다.",
            "Control に運行が見つからないか、運行一覧を一時的に利用できません。",
            "Control 中找不到该行程，或行程列表暂时不可用。",
        ],
        ProblemKind::GatewayOrder => [
            "Через інтернет: спершу підключіть усі рейси, потім увімкніть Gateway; щоб додати рейс, спершу вимкніть його.",
            "Internet mode: connect all trips first, then switch the Gateway on; switch it off before adding a trip.",
            "Internetbetrieb: zuerst alle Touren verbinden, dann das Gateway einschalten; zum Hinzufügen erst ausschalten.",
            "Mode internet : connectez d’abord tous les trajets, puis activez le Gateway ; désactivez-le avant d’ajouter un trajet.",
            "Modo internet: conecte primero todos los viajes y luego active el Gateway; desactívelo antes de añadir un viaje.",
            "인터넷 방식: 모든 운행을 먼저 연결한 뒤 Gateway를 켜세요. 운행을 추가하려면 먼저 끄세요.",
            "インターネット接続：先にすべての運行を接続してから Gateway を有効にします。追加する前に無効にしてください。",
            "互联网方式：先连接所有行程，再启用 Gateway；添加行程前请先停用。",
        ],
        ProblemKind::TripFolder => [
            "Не вдалося відкрити чи створити папку рейсу.",
            "The trip folder could not be opened or created.",
            "Der Tourordner konnte nicht geöffnet oder angelegt werden.",
            "Impossible d’ouvrir ou de créer le dossier du trajet.",
            "No se pudo abrir o crear la carpeta del viaje.",
            "운행 폴더를 열거나 만들 수 없습니다.",
            "運行フォルダーを開くことも作成することもできませんでした。",
            "无法打开或创建行程文件夹。",
        ],
        ProblemKind::TripData => [
            "Не вдалося прочитати чи зберегти дані рейсу.",
            "Trip data could not be read or saved.",
            "Tourdaten konnten nicht gelesen oder gespeichert werden.",
            "Impossible de lire ou d’enregistrer les données du trajet.",
            "No se pudieron leer o guardar los datos del viaje.",
            "운행 데이터를 읽거나 저장할 수 없습니다.",
            "運行データを読み込むか保存できませんでした。",
            "无法读取或保存行程数据。",
        ],
        ProblemKind::Sharing => [
            "Не вдалося змінити спосіб зв'язку з водіями. Перевірте адресу й порт.",
            "The driver connection could not be changed. Check the address and port.",
            "Die Fahrerverbindung ließ sich nicht ändern. Prüfen Sie Adresse und Port.",
            "Impossible de modifier la liaison avec les conducteurs. Vérifiez l’adresse et le port.",
            "No se pudo cambiar la conexión con los conductores. Revise la dirección y el puerto.",
            "운전자 연결 방식을 바꿀 수 없습니다. 주소와 포트를 확인하세요.",
            "ドライバーとの接続方法を変更できません。アドレスとポートを確認してください。",
            "无法更改与司机的连接方式，请检查地址和端口。",
        ],
        ProblemKind::Gateway => [
            "Не вдалося з'єднатися з Gateway. Перевірте його адресу й секрет.",
            "Could not connect to the Gateway. Check its address and secret.",
            "Keine Verbindung zum Gateway. Prüfen Sie Adresse und Geheimnis.",
            "Connexion au Gateway impossible. Vérifiez l’adresse et le secret.",
            "No se pudo conectar con el Gateway. Revise la dirección y el secreto.",
            "Gateway에 연결할 수 없습니다. 주소와 비밀키를 확인하세요.",
            "Gateway に接続できません。アドレスとシークレットを確認してください。",
            "无法连接 Gateway，请检查地址和密钥。",
        ],
        ProblemKind::Routing => [
            "Лінію дороги не пораховано. Перевірте налаштування Valhalla.",
            "The road line was not calculated. Check the Valhalla settings.",
            "Die Straßenlinie wurde nicht berechnet. Prüfen Sie die Valhalla-Einstellungen.",
            "Le tracé routier n’a pas été calculé. Vérifiez les réglages Valhalla.",
            "No se calculó la línea de carretera. Revise los ajustes de Valhalla.",
            "도로선을 계산하지 못했습니다. Valhalla 설정을 확인하세요.",
            "道路線を計算できませんでした。Valhalla の設定を確認してください。",
            "未能计算道路线，请检查 Valhalla 设置。",
        ],
        ProblemKind::Files => [
            "Не вдалося прочитати, передати або зберегти файл.",
            "The file could not be read, sent or saved.",
            "Die Datei konnte nicht gelesen, gesendet oder gespeichert werden.",
            "Impossible de lire, d’envoyer ou d’enregistrer le fichier.",
            "No se pudo leer, enviar o guardar el archivo.",
            "파일을 읽거나 보내거나 저장할 수 없습니다.",
            "ファイルの読み込み・送信・保存に失敗しました。",
            "无法读取、发送或保存文件。",
        ],
        ProblemKind::Window => [
            "Не вдалося відкрити чи показати вікно рейсу.",
            "The trip window could not be opened or shown.",
            "Das Tourfenster konnte nicht geöffnet oder angezeigt werden.",
            "Impossible d’ouvrir ou d’afficher la fenêtre du trajet.",
            "No se pudo abrir o mostrar la ventana del viaje.",
            "운행 창을 열거나 표시할 수 없습니다.",
            "運行ウィンドウを開くか表示できませんでした。",
            "无法打开或显示行程窗口。",
        ],
    }
}

fn has_cyrillic(text: &str) -> bool {
    text.chars().any(|ch| ('\u{0400}'..='\u{04ff}').contains(&ch))
}

/// The message to show for `problem` in the interface language `locale`
/// (`uk`, `en`, … `zh`). `fallback` names an unrecognised failure in that language.
#[must_use]
pub fn localize_problem(locale: &str, problem: &str, fallback: &str) -> String {
    let index = LOCALES
        .iter()
        .position(|code| locale.starts_with(code))
        .unwrap_or(1);
    let kind = classify_problem(problem);
    if kind == Some(ProblemKind::PortBusy) {
        return texts(ProblemKind::PortBusy)[index].to_owned();
    }
    // The original text is the most precise one when the operator can read it.
    if index == 0 || !has_cyrillic(problem) {
        return problem.to_owned();
    }
    let text = kind.map_or(fallback, |kind| texts(kind)[index]);
    let detail = problem
        .split_once(": ")
        .map(|(_, detail)| detail.trim())
        .filter(|detail| !detail.is_empty() && !has_cyrillic(detail));
    match detail {
        Some(detail) => format!("{text} ({detail})"),
        None => text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVICE: &str = include_str!("../../../apps/control-desktop/src-tauri/src/lib.rs");

    /// Messages the service can return: the first string after `Err(`,
    /// `map_err(` or `ok_or…(` in the same statement.
    fn service_messages() -> Vec<String> {
        let code = SERVICE.split("#[cfg(test)]").next().unwrap_or_default();
        let mut found = Vec::new();
        for token in ["Err(", "map_err(", "ok_or_else(", "ok_or("] {
            for (at, _) in code.match_indices(token) {
                let rest = &code[at + token.len()..];
                let statement = &rest[..rest.find(';').unwrap_or(rest.len())];
                let Some(start) = statement.find('"').filter(|start| *start < 200) else {
                    continue;
                };
                let Some(length) = statement[start + 1..].find('"') else {
                    continue;
                };
                let text = &statement[start + 1..start + 1 + length];
                if has_cyrillic(text) {
                    found.push(text.to_owned());
                }
            }
        }
        found
    }

    #[test]
    fn every_service_message_has_a_kind() {
        let messages = service_messages();
        assert!(messages.len() > 40, "only {} messages found", messages.len());
        let unknown: Vec<_> = messages
            .iter()
            .filter(|message| classify_problem(message).is_none())
            .collect();
        assert!(unknown.is_empty(), "no text for: {unknown:#?}");
    }

    #[test]
    fn a_busy_port_is_explained_in_every_language() {
        let raw = "Не вдалося запустити LAN test mode: Only one usage of each socket address \
                   (protocol/network address/port) is normally permitted. (os error 10048)";
        for locale in LOCALES {
            let shown = localize_problem(locale, raw, "fallback");
            assert!(!shown.contains("os error"), "{locale}: {shown}");
            assert!(shown.contains("Control"), "{locale}: {shown}");
        }
        assert_eq!(
            classify_problem("Не вдалося запустити loopback: Address already in use (os error 98)"),
            Some(ProblemKind::PortBusy)
        );
    }

    #[test]
    fn ukrainian_keeps_the_original_and_other_languages_get_a_translation() {
        let raw = "Не вдалося зберегти файл: Access is denied. (os error 5)";
        assert_eq!(localize_problem("uk", raw, "fallback"), raw);
        assert_eq!(
            localize_problem("en", raw, "fallback"),
            "The file could not be read, sent or saved. (Access is denied. (os error 5))"
        );
        assert_eq!(localize_problem("de", "Щось нове", "Fehler"), "Fehler");
        assert_eq!(localize_problem("fr", "HTTP 500", "Erreur"), "HTTP 500");
    }

    #[test]
    fn related_messages_do_not_fall_into_a_broader_kind() {
        for (message, kind) in [
            ("Некоректна LAN-адреса", ProblemKind::Sharing),
            ("Некоректні налаштування Gateway: x", ProblemKind::Gateway),
            ("Некоректна координата широта", ProblemKind::InvalidInput),
            ("Control не прийняв файл: x", ProblemKind::Files),
        ] {
            assert_eq!(classify_problem(message), Some(kind), "{message}");
        }
    }
}
