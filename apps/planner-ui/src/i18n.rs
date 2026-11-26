#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiLocale {
    Uk,
    En,
    De,
    Fr,
    Es,
    Ko,
    Ja,
    Zh,
}

impl UiLocale {
    pub fn parse(value: &str) -> Self {
        let value = value.to_ascii_lowercase();
        if value.starts_with("uk") {
            Self::Uk
        } else if value.starts_with("de") {
            Self::De
        } else if value.starts_with("fr") {
            Self::Fr
        } else if value.starts_with("es") {
            Self::Es
        } else if value.starts_with("ko") {
            Self::Ko
        } else if value.starts_with("ja") {
            Self::Ja
        } else if value.starts_with("zh") {
            Self::Zh
        } else {
            Self::En
        }
    }

    pub const fn code(self) -> &'static str {
        match self {
            Self::Uk => "uk",
            Self::En => "en",
            Self::De => "de",
            Self::Fr => "fr",
            Self::Es => "es",
            Self::Ko => "ko",
            Self::Ja => "ja",
            Self::Zh => "zh-Hans",
        }
    }
}

pub struct PlannerStrings {
    pub app_title: &'static str,
    pub subtitle: &'static str,
    pub language: &'static str,
    pub manual_route: &'static str,
    pub revision: &'static str,
    pub point_title: &'static str,
    pub latitude: &'static str,
    pub longitude: &'static str,
    pub add_point: &'static str,
    pub reset_route: &'static str,
    pub empty_route: &'static str,
    pub move_up: &'static str,
    pub move_down: &'static str,
    pub remove: &'static str,
    pub route_note: &'static str,
    pub added: &'static str,
    pub removed: &'static str,
    pub reordered: &'static str,
    pub reset_done: &'static str,
    pub invalid_coordinates: &'static str,
}

static UK: PlannerStrings = PlannerStrings {
    app_title: "Cargo Dispatch — Планувальник",
    subtitle: "Ручний маршрут",
    language: "Мова",
    manual_route: "Чернетка маршруту",
    revision: "Ревізія",
    point_title: "Назва точки",
    latitude: "Широта",
    longitude: "Довгота",
    add_point: "Додати точку",
    reset_route: "Новий маршрут",
    empty_route: "Маршрут поки порожній.",
    move_up: "Вище",
    move_down: "Нижче",
    remove: "Видалити",
    route_note: "Кожна зміна порядку й складу точок проходить через Planner core та збільшує ревізію.",
    added: "Точку додано.",
    removed: "Точку видалено.",
    reordered: "Порядок точок змінено.",
    reset_done: "Створено нову чернетку маршруту.",
    invalid_coordinates: "Перевірте назву та координати точки.",
};
static EN: PlannerStrings = PlannerStrings {
    app_title: "Cargo Dispatch — Planner",
    subtitle: "Manual route",
    language: "Language",
    manual_route: "Route draft",
    revision: "Revision",
    point_title: "Point name",
    latitude: "Latitude",
    longitude: "Longitude",
    add_point: "Add point",
    reset_route: "New route",
    empty_route: "The route is empty.",
    move_up: "Up",
    move_down: "Down",
    remove: "Remove",
    route_note: "Every point/order edit goes through Planner core and increments the revision.",
    added: "Point added.",
    removed: "Point removed.",
    reordered: "Point order changed.",
    reset_done: "A new route draft was created.",
    invalid_coordinates: "Check the point name and coordinates.",
};
static DE: PlannerStrings = PlannerStrings {
    app_title: "Cargo Dispatch — Planner",
    subtitle: "Manuelle Route",
    language: "Sprache",
    manual_route: "Routenentwurf",
    revision: "Revision",
    point_title: "Punktname",
    latitude: "Breitengrad",
    longitude: "Längengrad",
    add_point: "Punkt hinzufügen",
    reset_route: "Neue Route",
    empty_route: "Die Route ist leer.",
    move_up: "Nach oben",
    move_down: "Nach unten",
    remove: "Entfernen",
    route_note: "Jede Änderung an Punkten oder Reihenfolge läuft durch den Planner-Core und erhöht die Revision.",
    added: "Punkt hinzugefügt.",
    removed: "Punkt entfernt.",
    reordered: "Punktreihenfolge geändert.",
    reset_done: "Ein neuer Routenentwurf wurde erstellt.",
    invalid_coordinates: "Punktname und Koordinaten prüfen.",
};
static FR: PlannerStrings = PlannerStrings {
    app_title: "Cargo Dispatch — Planner",
    subtitle: "Itinéraire manuel",
    language: "Langue",
    manual_route: "Brouillon d’itinéraire",
    revision: "Révision",
    point_title: "Nom du point",
    latitude: "Latitude",
    longitude: "Longitude",
    add_point: "Ajouter le point",
    reset_route: "Nouvel itinéraire",
    empty_route: "L’itinéraire est vide.",
    move_up: "Monter",
    move_down: "Descendre",
    remove: "Supprimer",
    route_note: "Chaque modification des points ou de leur ordre passe par le cœur Planner et augmente la révision.",
    added: "Point ajouté.",
    removed: "Point supprimé.",
    reordered: "Ordre des points modifié.",
    reset_done: "Un nouveau brouillon a été créé.",
    invalid_coordinates: "Vérifiez le nom et les coordonnées du point.",
};
static ES: PlannerStrings = PlannerStrings {
    app_title: "Cargo Dispatch — Planner",
    subtitle: "Ruta manual",
    language: "Idioma",
    manual_route: "Borrador de ruta",
    revision: "Revisión",
    point_title: "Nombre del punto",
    latitude: "Latitud",
    longitude: "Longitud",
    add_point: "Añadir punto",
    reset_route: "Nueva ruta",
    empty_route: "La ruta está vacía.",
    move_up: "Subir",
    move_down: "Bajar",
    remove: "Eliminar",
    route_note: "Cada cambio de puntos u orden pasa por el núcleo Planner y aumenta la revisión.",
    added: "Punto añadido.",
    removed: "Punto eliminado.",
    reordered: "Orden de puntos cambiado.",
    reset_done: "Se creó un nuevo borrador.",
    invalid_coordinates: "Revisa el nombre y las coordenadas del punto.",
};
static KO: PlannerStrings = PlannerStrings {
    app_title: "Cargo Dispatch — Planner",
    subtitle: "수동 경로",
    language: "언어",
    manual_route: "경로 초안",
    revision: "리비전",
    point_title: "지점 이름",
    latitude: "위도",
    longitude: "경도",
    add_point: "지점 추가",
    reset_route: "새 경로",
    empty_route: "경로가 비어 있습니다.",
    move_up: "위로",
    move_down: "아래로",
    remove: "삭제",
    route_note: "지점과 순서의 모든 변경은 Planner core를 거치며 리비전이 증가합니다.",
    added: "지점을 추가했습니다.",
    removed: "지점을 삭제했습니다.",
    reordered: "지점 순서를 변경했습니다.",
    reset_done: "새 경로 초안을 만들었습니다.",
    invalid_coordinates: "지점 이름과 좌표를 확인하세요.",
};
static JA: PlannerStrings = PlannerStrings {
    app_title: "Cargo Dispatch — Planner",
    subtitle: "手動ルート",
    language: "言語",
    manual_route: "ルート下書き",
    revision: "リビジョン",
    point_title: "地点名",
    latitude: "緯度",
    longitude: "経度",
    add_point: "地点を追加",
    reset_route: "新しいルート",
    empty_route: "ルートは空です。",
    move_up: "上へ",
    move_down: "下へ",
    remove: "削除",
    route_note: "地点や順序の変更はすべて Planner core を通り、リビジョンが増加します。",
    added: "地点を追加しました。",
    removed: "地点を削除しました。",
    reordered: "地点順を変更しました。",
    reset_done: "新しいルート下書きを作成しました。",
    invalid_coordinates: "地点名と座標を確認してください。",
};
static ZH: PlannerStrings = PlannerStrings {
    app_title: "Cargo Dispatch — Planner",
    subtitle: "手动路线",
    language: "语言",
    manual_route: "路线草稿",
    revision: "修订",
    point_title: "站点名称",
    latitude: "纬度",
    longitude: "经度",
    add_point: "添加站点",
    reset_route: "新路线",
    empty_route: "路线为空。",
    move_up: "上移",
    move_down: "下移",
    remove: "删除",
    route_note: "每次站点或顺序修改都会经过 Planner core 并增加修订号。",
    added: "已添加站点。",
    removed: "已删除站点。",
    reordered: "已更改站点顺序。",
    reset_done: "已创建新的路线草稿。",
    invalid_coordinates: "请检查站点名称和坐标。",
};

pub fn strings(locale: UiLocale) -> &'static PlannerStrings {
    match locale {
        UiLocale::Uk => &UK,
        UiLocale::En => &EN,
        UiLocale::De => &DE,
        UiLocale::Fr => &FR,
        UiLocale::Es => &ES,
        UiLocale::Ko => &KO,
        UiLocale::Ja => &JA,
        UiLocale::Zh => &ZH,
    }
}
