//! What the planner reads when the Planner service reports a failure.
//!
//! The service describes failures in English, sometimes with a technical tail.
//! This module sorts them into a few kinds and gives each kind a text in the
//! other interface languages.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemKind {
    PointInput,
    RouteOrder,
    RouteTooShort,
    RoutingFields,
    Routing,
    PreviewStale,
    PackageExists,
    PackagePath,
    Cargo,
    Export,
    Storage,
    Recovery,
    Internal,
    InputFile,
    Orders,
    MapPoints,
    Figures,
    Place,
    PlaceNetwork,
}

/// Fragments of service messages. The first match wins.
const FRAGMENTS: &[(&str, ProblemKind)] = &[
    ("input file", ProblemKind::InputFile),
    ("Input file", ProblemKind::InputFile),
    ("planning input", ProblemKind::Storage),
    ("vehicle data", ProblemKind::Orders),
    ("Vehicle type", ProblemKind::Orders),
    ("map service takes", ProblemKind::MapPoints),
    ("The typed", ProblemKind::Figures),
    ("Map link has no point", ProblemKind::Place),
    ("Map link could not be opened", ProblemKind::PlaceNetwork),
    ("order selected", ProblemKind::Orders),
    ("orders for one calculation", ProblemKind::Orders),
    ("Order has no coordinates", ProblemKind::Orders),
    ("of every order", ProblemKind::Orders),
    ("Planner journal contains", ProblemKind::Recovery),
    ("Planner journal removes", ProblemKind::Recovery),
    ("journal export revision", ProblemKind::Recovery),
    ("journal trip version", ProblemKind::Recovery),
    ("journal serial", ProblemKind::Recovery),
    ("Unsupported Planner journal", ProblemKind::Recovery),
    ("Could not recover", ProblemKind::Recovery),
    ("rejected recovered", ProblemKind::Recovery),
    ("state unavailable", ProblemKind::Internal),
    ("version overflow", ProblemKind::Internal),
    ("id overflow", ProblemKind::Internal),
    ("System clock", ProblemKind::Internal),
    ("System time", ProblemKind::Internal),
    ("Planner journal", ProblemKind::Storage),
    ("Planner data folder", ProblemKind::Storage),
    ("operator id", ProblemKind::Storage),
    ("Reorder must", ProblemKind::RouteOrder),
    ("Unknown point id", ProblemKind::RouteOrder),
    ("point metadata is missing", ProblemKind::RouteOrder),
    ("at least two ordered points", ProblemKind::RouteTooShort),
    ("Point title is required", ProblemKind::PointInput),
    ("Latitude must", ProblemKind::PointInput),
    ("Longitude must", ProblemKind::PointInput),
    ("Invalid Planner point", ProblemKind::PointInput),
    ("too many points", ProblemKind::PointInput),
    ("e7 coordinate", ProblemKind::PointInput),
    ("in routing geometry", ProblemKind::PointInput),
    ("address is required", ProblemKind::RoutingFields),
    ("dataset id is required", ProblemKind::RoutingFields),
    ("timeout must be", ProblemKind::RoutingFields),
    ("Routing preview requires", ProblemKind::PreviewStale),
    ("preview is stale", ProblemKind::PreviewStale),
    ("preview engine version", ProblemKind::PreviewStale),
    ("preview map dataset", ProblemKind::PreviewStale),
    ("preview geometry", ProblemKind::PreviewStale),
    ("Valhalla", ProblemKind::Routing),
    ("routing preview request", ProblemKind::Routing),
    ("Planner routing profile", ProblemKind::Routing),
    ("Package path already exists", ProblemKind::PackageExists),
    ("package history", ProblemKind::Export),
    ("export history", ProblemKind::Export),
    ("Package path", ProblemKind::PackagePath),
    ("package path", ProblemKind::PackagePath),
    ("package folder", ProblemKind::PackagePath),
    ("package save dialog", ProblemKind::PackagePath),
    ("cargo", ProblemKind::Cargo),
    ("Cargo ", ProblemKind::Cargo),
    ("planning basis", ProblemKind::Export),
    ("create trip", ProblemKind::Export),
    ("base trip version", ProblemKind::Export),
    ("package header", ProblemKind::Export),
    ("Could not attach", ProblemKind::Export),
    ("export trip package", ProblemKind::Export),
];

#[must_use]
pub fn classify_problem(message: &str) -> Option<ProblemKind> {
    FRAGMENTS
        .iter()
        .find(|(fragment, _)| message.contains(fragment))
        .map(|(_, kind)| *kind)
}

/// Texts in the order of `OTHER_LOCALES`.
fn texts(kind: ProblemKind) -> [&'static str; 7] {
    match kind {
        ProblemKind::PointInput => [
            "Перевірте назву та координати точки.",
            "Prüfen Sie Name und Koordinaten des Punkts.",
            "Vérifiez le nom et les coordonnées du point.",
            "Revise el nombre y las coordenadas del punto.",
            "지점 이름과 좌표를 확인하세요.",
            "地点の名前と座標を確認してください。",
            "请检查地点名称和坐标。",
        ],
        ProblemKind::RouteOrder => [
            "Список точок змінився. Повторіть дію.",
            "Die Punktliste hat sich geändert. Wiederholen Sie die Aktion.",
            "La liste des points a changé. Recommencez.",
            "La lista de puntos ha cambiado. Repita la acción.",
            "지점 목록이 바뀌었습니다. 다시 시도하세요.",
            "地点の一覧が変わりました。もう一度操作してください。",
            "地点列表已变化，请重试。",
        ],
        ProblemKind::RouteTooShort => [
            "Для маршруту потрібні щонайменше дві точки.",
            "Eine Route braucht mindestens zwei Punkte.",
            "Un itinéraire exige au moins deux points.",
            "La ruta necesita al menos dos puntos.",
            "경로에는 지점이 두 곳 이상 필요합니다.",
            "経路には2地点以上が必要です。",
            "路线至少需要两个地点。",
        ],
        ProblemKind::RoutingFields => [
            "Вкажіть адресу Valhalla, набір карт і правильні тайм-аути.",
            "Geben Sie Valhalla-Adresse, Kartensatz und gültige Zeitlimits an.",
            "Indiquez l’adresse Valhalla, le jeu de cartes et des délais valides.",
            "Indique la dirección de Valhalla, el conjunto de mapas y tiempos de espera válidos.",
            "Valhalla 주소, 지도 데이터, 올바른 제한 시간을 입력하세요.",
            "Valhalla のアドレス、地図データ、正しいタイムアウトを指定してください。",
            "请填写 Valhalla 地址、地图数据集和有效的超时时间。",
        ],
        ProblemKind::Routing => [
            "Маршрут не пораховано. Перевірте Valhalla та параметри авто.",
            "Die Route wurde nicht berechnet. Prüfen Sie Valhalla und die Fahrzeugdaten.",
            "L’itinéraire n’a pas été calculé. Vérifiez Valhalla et les paramètres du véhicule.",
            "No se calculó la ruta. Revise Valhalla y los datos del vehículo.",
            "경로를 계산하지 못했습니다. Valhalla와 차량 제원을 확인하세요.",
            "経路を計算できませんでした。Valhalla と車両条件を確認してください。",
            "未能计算路线，请检查 Valhalla 和车辆参数。",
        ],
        ProblemKind::PreviewStale => [
            "Попередній маршрут застарів. Порахуйте його ще раз.",
            "Die Routenvorschau ist veraltet. Berechnen Sie sie erneut.",
            "L’aperçu de l’itinéraire est périmé. Recalculez-le.",
            "La vista previa de la ruta está desactualizada. Calcúlela de nuevo.",
            "경로 미리보기가 오래되었습니다. 다시 계산하세요.",
            "経路プレビューが古くなっています。再計算してください。",
            "路线预览已过期，请重新计算。",
        ],
        ProblemKind::PackageExists => [
            "Такий файл уже є. Виберіть іншу назву: наявний файл не перезаписується.",
            "Die Datei gibt es schon. Wählen Sie einen anderen Namen; nichts wird überschrieben.",
            "Ce fichier existe déjà. Choisissez un autre nom : rien n’est écrasé.",
            "El archivo ya existe. Elija otro nombre: no se sobrescribe nada.",
            "같은 파일이 이미 있습니다. 다른 이름을 선택하세요. 기존 파일은 덮어쓰지 않습니다.",
            "同じファイルがあります。別の名前を選んでください。上書きはしません。",
            "文件已存在，请换一个名称；不会覆盖现有文件。",
        ],
        ProblemKind::PackagePath => [
            "Не вдалося використати це місце для пакета. Виберіть інше.",
            "Dieser Speicherort lässt sich nicht für das Paket nutzen. Wählen Sie einen anderen.",
            "Cet emplacement ne convient pas au paquet. Choisissez-en un autre.",
            "No se puede usar esta ubicación para el paquete. Elija otra.",
            "이 위치에는 패키지를 저장할 수 없습니다. 다른 위치를 선택하세요.",
            "この場所にはパッケージを保存できません。別の場所を選んでください。",
            "无法在此位置保存行程包，请另选位置。",
        ],
        ProblemKind::Cargo => [
            "Перевірте вантажі: потрібен щонайменше один, кожен із назвою.",
            "Prüfen Sie die Ladung: mindestens eine, jede mit Bezeichnung.",
            "Vérifiez les marchandises : au moins une, chacune nommée.",
            "Revise la carga: al menos una, cada una con nombre.",
            "화물을 확인하세요. 하나 이상이어야 하며 각각 이름이 필요합니다.",
            "貨物を確認してください。1件以上必要で、それぞれ名称が必要です。",
            "请检查货物：至少一项，且每项都要有名称。",
        ],
        ProblemKind::Export => [
            "Не вдалося створити пакет рейсу. Перевірте дані й повторіть.",
            "Das Tourpaket wurde nicht erstellt. Prüfen Sie die Daten und wiederholen Sie.",
            "Le paquet de trajet n’a pas été créé. Vérifiez les données et recommencez.",
            "No se creó el paquete de viaje. Revise los datos y repita.",
            "운행 패키지를 만들지 못했습니다. 데이터를 확인하고 다시 시도하세요.",
            "運行パッケージを作成できませんでした。内容を確認してやり直してください。",
            "未能生成行程包，请检查数据后重试。",
        ],
        ProblemKind::Storage => [
            "Не вдалося прочитати або записати дані Planner на диск.",
            "Planner-Daten konnten nicht gelesen oder gespeichert werden.",
            "Impossible de lire ou d’écrire les données de Planner sur le disque.",
            "No se pudieron leer o escribir los datos de Planner en el disco.",
            "Planner 데이터를 디스크에서 읽거나 쓸 수 없습니다.",
            "Planner のデータをディスクで読み書きできませんでした。",
            "无法在磁盘上读写 Planner 数据。",
        ],
        ProblemKind::Recovery => [
            "Збережену чернетку не вдалося відновити: її записи пошкоджені.",
            "Der gespeicherte Entwurf ließ sich nicht wiederherstellen: Einträge sind beschädigt.",
            "Le brouillon enregistré n’a pas pu être rétabli : des entrées sont endommagées.",
            "No se pudo recuperar el borrador guardado: hay registros dañados.",
            "저장된 초안을 복원하지 못했습니다. 기록이 손상되었습니다.",
            "保存した下書きを復元できません。記録が破損しています。",
            "无法恢复已保存的草稿：记录已损坏。",
        ],
        ProblemKind::InputFile => [
            "Файл заявок не вдалося прочитати чи записати. Перевірте його вміст за описом формату.",
            "Die Auftragsdatei konnte nicht gelesen oder geschrieben werden. Prüfen Sie ihren Inhalt anhand der Formatbeschreibung.",
            "Impossible de lire ou d’écrire le fichier des commandes. Vérifiez son contenu selon la description du format.",
            "No se pudo leer o escribir el archivo de pedidos. Revise su contenido según la descripción del formato.",
            "주문 파일을 읽거나 쓸 수 없습니다. 형식 설명에 따라 내용을 확인하세요.",
            "注文ファイルを読み書きできません。形式の説明に沿って内容を確認してください。",
            "无法读取或写入订单文件。请按格式说明检查内容。",
        ],
        ProblemKind::Orders => [
            "Перевірте вибрані заявки: дані авто, координати, обидві зупинки кожної та не більше 12 за раз.",
            "Prüfen Sie die gewählten Aufträge: Fahrzeugdaten, Koordinaten, beide Stopps je Auftrag, höchstens 12 auf einmal.",
            "Vérifiez les commandes choisies : données du véhicule, coordonnées, les deux arrêts de chacune, 12 au maximum.",
            "Revise los pedidos elegidos: datos del vehículo, coordenadas, las dos paradas de cada uno y 12 como máximo.",
            "선택한 주문을 확인하세요: 차량 정보, 좌표, 주문별 두 지점, 한 번에 최대 12건.",
            "選んだ注文を確認してください：車両情報、座標、各注文の2地点、一度に12件まで。",
            "请检查所选订单：车辆信息、坐标、每单两个地点，一次最多 12 单。",
        ],
        ProblemKind::MapPoints => [
            "Забагато точок для цього сервісу: Google Maps приймає не більше 11 разом із місцем авто.",
            "Zu viele Punkte: Google Maps nimmt höchstens 11 einschließlich Fahrzeugstandort an.",
            "Trop de points : Google Maps en accepte 11 au maximum, position du véhicule comprise.",
            "Demasiados puntos: Google Maps admite 11 como máximo, incluida la posición del vehículo.",
            "지점이 너무 많습니다. Google 지도는 차량 위치를 포함해 최대 11곳입니다.",
            "地点が多すぎます。Google マップは車両位置を含めて11地点までです。",
            "地点过多：Google 地图最多 11 个（含车辆位置）。",
        ],
        ProblemKind::Figures => [
            "Перевірте внесені кілометри та час (год:хв) або очистіть поле.",
            "Prüfen Sie die eingegebenen Kilometer und die Zeit (Std:Min) oder leeren Sie das Feld.",
            "Vérifiez les kilomètres et le temps saisis (h:min) ou videz le champ.",
            "Revise los kilómetros y el tiempo introducidos (h:min) o vacíe el campo.",
            "입력한 거리와 시간(시:분)을 확인하거나 칸을 비우세요.",
            "入力した距離と時間（時:分）を確認するか、欄を空にしてください。",
            "请检查输入的公里数和时间（时:分），或清空该栏。",
        ],
        ProblemKind::Place => [
            "Не знайшов точку. Вставте посилання на місце з Google Maps або його координати.",
            "Kein Punkt gefunden. Fügen Sie den Link zum Ort aus Google Maps oder seine Koordinaten ein.",
            "Aucun point trouvé. Collez le lien du lieu depuis Google Maps ou ses coordonnées.",
            "No se encontró el punto. Pegue el enlace del lugar de Google Maps o sus coordenadas.",
            "지점을 찾지 못했습니다. Google 지도의 장소 링크나 좌표를 붙여넣으세요.",
            "地点が見つかりません。Google マップの場所のリンクか座標を貼り付けてください。",
            "未找到地点。请粘贴 Google 地图中的地点链接或坐标。",
        ],
        ProblemKind::PlaceNetwork => [
            "Не вдалося відкрити коротке посилання. Перевірте інтернет або вставте координати.",
            "Der Kurzlink ließ sich nicht öffnen. Prüfen Sie das Internet oder fügen Sie die Koordinaten ein.",
            "Impossible d’ouvrir le lien court. Vérifiez Internet ou collez les coordonnées.",
            "No se pudo abrir el enlace corto. Revise Internet o pegue las coordenadas.",
            "짧은 링크를 열 수 없습니다. 인터넷을 확인하거나 좌표를 붙여넣으세요.",
            "短縮リンクを開けません。インターネットを確認するか、座標を貼り付けてください。",
            "无法打开短链接。请检查网络或粘贴坐标。",
        ],
        ProblemKind::Internal => [
            "Внутрішня помилка Planner. Перезапустіть програму.",
            "Interner Planner-Fehler. Starten Sie das Programm neu.",
            "Erreur interne de Planner. Redémarrez le programme.",
            "Error interno de Planner. Reinicie el programa.",
            "Planner 내부 오류입니다. 프로그램을 다시 시작하세요.",
            "Planner の内部エラーです。再起動してください。",
            "Planner 内部错误，请重启程序。",
        ],
    }
}

/// Languages other than English, in the order of the texts.
const OTHER_LOCALES: [&str; 7] = ["uk", "de", "fr", "es", "ko", "ja", "zh"];

/// The message to show for `problem` in the interface language `locale`.
/// English keeps the original, and so does an unrecognised failure.
#[must_use]
pub fn localize_problem(locale: &str, problem: &str) -> String {
    let Some(index) = OTHER_LOCALES
        .iter()
        .position(|code| locale.starts_with(code))
    else {
        return problem.to_owned();
    };
    let Some(kind) = classify_problem(problem) else {
        return problem.to_owned();
    };
    let text = texts(kind)[index];
    match problem.split_once(": ").map(|(_, detail)| detail.trim()) {
        Some(detail) if !detail.is_empty() => format!("{text} ({detail})"),
        _ => text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVICE: &str = include_str!("../../../apps/planner-desktop/src-tauri/src/lib.rs");

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
                if text.contains(' ') {
                    found.push(text.to_owned());
                }
            }
        }
        found
    }

    #[test]
    fn every_service_message_has_a_kind() {
        let messages = service_messages();
        assert!(
            messages.len() > 50,
            "only {} messages found",
            messages.len()
        );
        let unknown: Vec<_> = messages
            .iter()
            .filter(|message| classify_problem(message).is_none())
            .collect();
        assert!(unknown.is_empty(), "no text for: {unknown:#?}");
    }

    #[test]
    fn english_keeps_the_original_and_other_languages_get_a_translation() {
        let raw = "Could not write Planner journal: Access is denied. (os error 5)";
        assert_eq!(localize_problem("en", raw), raw);
        assert_eq!(
            localize_problem("uk", raw),
            "Не вдалося прочитати або записати дані Planner на диск. (Access is denied. (os error 5))"
        );
        assert_eq!(
            localize_problem("zh-Hans", "Package path already exists"),
            "文件已存在，请换一个名称；不会覆盖现有文件。"
        );
        assert_eq!(localize_problem("de", "something new"), "something new");
    }

    #[test]
    fn related_messages_do_not_fall_into_a_broader_kind() {
        for (message, kind) in [
            ("Planner trip version overflow", ProblemKind::Internal),
            (
                "Planner journal contains invalid point id: x",
                ProblemKind::Recovery,
            ),
            ("Package path already exists", ProblemKind::PackageExists),
            ("Package path is required", ProblemKind::PackagePath),
            ("Could not attach routing profile: x", ProblemKind::Export),
        ] {
            assert_eq!(classify_problem(message), Some(kind), "{message}");
        }
    }
}
