use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

pub const SYNTHETIC_ROW_COUNT: usize = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestStatus {
    New,
    Contacting,
    Reserved,
    Assigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    Id,
    AgeMinutes,
    Origin,
    Destination,
    WeightKg,
    RateEur,
    Confidence,
    Status,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadRequest {
    pub id: u64,
    pub age_minutes: u32,
    pub source: String,
    pub origin: String,
    pub destination: String,
    pub pickup_day: u16,
    pub weight_kg: u32,
    pub pallets: u16,
    pub rate_eur: u32,
    pub confidence_percent: u8,
    pub status: RequestStatus,
    pub reserved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntakeMode {
    Manual,
    Phone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditorFieldSection {
    Common,
    PhoneOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorField {
    pub key: String,
    pub label: String,
    pub value: String,
    pub required: bool,
    pub section: EditorFieldSection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestDraft {
    pub request_id: u64,
    pub mode: IntakeMode,
    pub source: String,
    pub confidence_percent: u8,
    pub fields: Vec<EditorField>,
}

impl RequestDraft {
    pub fn field_value(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|field| field.key == key)
            .map(|field| field.value.as_str())
    }

    pub fn set_field(&mut self, key: &str, value: impl Into<String>) -> bool {
        let Some(field) = self.fields.iter_mut().find(|field| field.key == key) else {
            return false;
        };
        field.value = value.into();
        true
    }

    pub fn visible_fields(&self) -> Vec<EditorField> {
        self.fields
            .iter()
            .filter(|field| {
                field.section == EditorFieldSection::Common || self.mode == IntakeMode::Phone
            })
            .cloned()
            .collect()
    }

    pub fn visible_field_count(&self) -> usize {
        self.visible_fields().len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub field_key: String,
    pub message: String,
}

impl ValidationIssue {
    fn new(field_key: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field_key: field_key.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UiLanguage {
    Uk,
    En,
    De,
}

impl UiLanguage {
    pub fn code(self) -> &'static str {
        match self {
            Self::Uk => "uk",
            Self::En => "en",
            Self::De => "de",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "uk" => Some(Self::Uk),
            "en" => Some(Self::En),
            "de" => Some(Self::De),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiTextKey {
    Language,
    PlannerEyebrow,
    PlannerTitle,
    SyntheticRequests,
    Requests,
    Reserved,
    Assigned,
    Selected,
    Filter,
    SortAge,
    SortRate,
    VisibleRows,
    Select,
    Open,
    Age,
    Source,
    Origin,
    Destination,
    Rate,
    Confidence,
    EditRequest,
    RequestEditor,
    SourceLabel,
    ConfidenceLabel,
    UnsavedChanges,
    SavedState,
    Saved,
    Manual,
    PhoneOrder,
    CorrectForm,
    ChangesNotSaved,
    CloseWithoutSaving,
    DiscardClose,
    ContinueEditing,
    Save,
    Cancel,
    Close,
    WorkingRoute,
    RouteDraft,
    FormatSample,
    DriverTitle,
    CurrentPoint,
    Documents,
    Required,
    Optional,
    AddFile,
    NoFile,
    DownloadDocument,
    Phone,
    CheckLocation,
    OpenNavigator,
    ChangeStatus,
    Driving,
    Arrived,
    Done,
    FieldRequired,
    InvalidValue,
    PhoneRequired,
    OriginDestinationDiffer,
}

pub fn ui_text(language: UiLanguage, key: UiTextKey) -> &'static str {
    use UiLanguage::{De, En, Uk};
    use UiTextKey::*;

    match (language, key) {
        (Uk, Language) => "Мова",
        (Uk, PlannerEyebrow) => "Cargo dispatch · перевірка інтерфейсу",
        (Uk, PlannerTitle) => "Пул заявок і редагування",
        (Uk, SyntheticRequests) => "тестових заявок",
        (Uk, Requests) => "заявки",
        (Uk, Reserved) => "зарезервовано",
        (Uk, Assigned) => "призначено",
        (Uk, Selected) => "вибрано",
        (Uk, Filter) => "Фільтр",
        (Uk, SortAge) => "За віком",
        (Uk, SortRate) => "За ставкою",
        (Uk, VisibleRows) => "видимих рядків",
        (Uk, Select) => "Вибір",
        (Uk, Open) => "Відкрити",
        (Uk, Age) => "Вік",
        (Uk, Source) => "Джерело",
        (Uk, Origin) => "Звідки",
        (Uk, Destination) => "Куди",
        (Uk, Rate) => "Ставка €",
        (Uk, Confidence) => "Надійність",
        (Uk, EditRequest) => "Редагувати заявку",
        (Uk, RequestEditor) => "Редактор заявки",
        (Uk, SourceLabel) => "Джерело:",
        (Uk, ConfidenceLabel) => "Надійність:",
        (Uk, UnsavedChanges) => "Незбережені зміни",
        (Uk, SavedState) => "Збережений стан",
        (Uk, Saved) => "Збережено",
        (Uk, Manual) => "Вручну",
        (Uk, PhoneOrder) => "Телефонне замовлення",
        (Uk, CorrectForm) => "Виправте дані форми:",
        (Uk, ChangesNotSaved) => "Зміни не збережені.",
        (Uk, CloseWithoutSaving) => "Закрити без збереження чи продовжити редагування.",
        (Uk, DiscardClose) => "Відкинути й закрити",
        (Uk, ContinueEditing) => "Продовжити редагування",
        (Uk, Save) => "Зберегти",
        (Uk, Cancel) => "Скасувати",
        (Uk, Close) => "Закрити",
        (Uk, WorkingRoute) => "Робочий маршрут",
        (Uk, RouteDraft) => "Чернетка маршруту",
        (Uk, FormatSample) => "Формат числа і дати",
        (Uk, DriverTitle) => "Водій",
        (Uk, CurrentPoint) => "Поточна точка",
        (Uk, Documents) => "Документи",
        (Uk, Required) => "обов’язковий",
        (Uk, Optional) => "за потреби",
        (Uk, AddFile) => "Додати фото / PDF / файл",
        (Uk, NoFile) => "Файл ще не вибрано",
        (Uk, DownloadDocument) => "Завантажити документ",
        (Uk, Phone) => "Телефон",
        (Uk, CheckLocation) => "Перевірити моє місце",
        (Uk, OpenNavigator) => "Відкрити навігатор",
        (Uk, ChangeStatus) => "Змінити статус",
        (Uk, Driving) => "У дорозі",
        (Uk, Arrived) => "Прибув",
        (Uk, Done) => "Завдання виконано",
        (Uk, FieldRequired) => "обов’язкове поле",
        (Uk, InvalidValue) => "некоректне значення",
        (Uk, PhoneRequired) => "для телефонного замовлення потрібен номер",
        (Uk, OriginDestinationDiffer) => "місце відправлення і призначення мають відрізнятися",

        (En, Language) => "Language",
        (En, PlannerEyebrow) => "Cargo dispatch · UI probe",
        (En, PlannerTitle) => "Planner pool and request editor",
        (En, SyntheticRequests) => "synthetic requests",
        (En, Requests) => "requests",
        (En, Reserved) => "reserved",
        (En, Assigned) => "assigned",
        (En, Selected) => "selected",
        (En, Filter) => "Filter",
        (En, SortAge) => "Sort age",
        (En, SortRate) => "Sort rate",
        (En, VisibleRows) => "visible rows",
        (En, Select) => "Select",
        (En, Open) => "Open",
        (En, Age) => "Age",
        (En, Source) => "Source",
        (En, Origin) => "Origin",
        (En, Destination) => "Destination",
        (En, Rate) => "Rate €",
        (En, Confidence) => "Confidence",
        (En, EditRequest) => "Edit request",
        (En, RequestEditor) => "Request editor",
        (En, SourceLabel) => "Source:",
        (En, ConfidenceLabel) => "Confidence:",
        (En, UnsavedChanges) => "Unsaved changes",
        (En, SavedState) => "Saved state",
        (En, Saved) => "Saved",
        (En, Manual) => "Manual",
        (En, PhoneOrder) => "Phone order",
        (En, CorrectForm) => "Please correct the form:",
        (En, ChangesNotSaved) => "Changes are not saved.",
        (En, CloseWithoutSaving) => "Close without saving, or continue editing.",
        (En, DiscardClose) => "Discard and close",
        (En, ContinueEditing) => "Continue editing",
        (En, Save) => "Save",
        (En, Cancel) => "Cancel",
        (En, Close) => "Close",
        (En, WorkingRoute) => "Working route",
        (En, RouteDraft) => "Route draft",
        (En, FormatSample) => "Number and date format",
        (En, DriverTitle) => "Driver",
        (En, CurrentPoint) => "Current point",
        (En, Documents) => "Documents",
        (En, Required) => "required",
        (En, Optional) => "optional",
        (En, AddFile) => "Add photo / PDF / file",
        (En, NoFile) => "No file selected",
        (En, DownloadDocument) => "Download document",
        (En, Phone) => "Phone",
        (En, CheckLocation) => "Check my location",
        (En, OpenNavigator) => "Open navigator",
        (En, ChangeStatus) => "Change status",
        (En, Driving) => "Driving",
        (En, Arrived) => "Arrived",
        (En, Done) => "Task completed",
        (En, FieldRequired) => "required field",
        (En, InvalidValue) => "invalid value",
        (En, PhoneRequired) => "phone order requires a contact number",
        (En, OriginDestinationDiffer) => "origin and destination must differ",

        (De, Language) => "Sprache",
        (De, PlannerEyebrow) => "Cargo dispatch · Oberflächenprüfung",
        (De, PlannerTitle) => "Auftragspool und Auftragsbearbeitung",
        (De, SyntheticRequests) => "Testaufträge",
        (De, Requests) => "Aufträge",
        (De, Reserved) => "reserviert",
        (De, Assigned) => "zugewiesen",
        (De, Selected) => "ausgewählt",
        (De, Filter) => "Filter",
        (De, SortAge) => "Nach Alter sortieren",
        (De, SortRate) => "Nach Preis sortieren",
        (De, VisibleRows) => "sichtbare Zeilen",
        (De, Select) => "Auswahl",
        (De, Open) => "Öffnen",
        (De, Age) => "Alter",
        (De, Source) => "Quelle",
        (De, Origin) => "Abholort",
        (De, Destination) => "Zielort",
        (De, Rate) => "Preis €",
        (De, Confidence) => "Verlässlichkeit",
        (De, EditRequest) => "Auftrag bearbeiten",
        (De, RequestEditor) => "Auftragsbearbeitung",
        (De, SourceLabel) => "Quelle:",
        (De, ConfidenceLabel) => "Verlässlichkeit:",
        (De, UnsavedChanges) => "Nicht gespeicherte Änderungen",
        (De, SavedState) => "Gespeicherter Stand",
        (De, Saved) => "Gespeichert",
        (De, Manual) => "Manuell",
        (De, PhoneOrder) => "Telefonisch aufgenommener Auftrag",
        (De, CorrectForm) => "Bitte korrigieren Sie die Eingaben:",
        (De, ChangesNotSaved) => "Änderungen sind nicht gespeichert.",
        (De, CloseWithoutSaving) => "Ohne Speichern schließen oder weiter bearbeiten.",
        (De, DiscardClose) => "Verwerfen und schließen",
        (De, ContinueEditing) => "Weiter bearbeiten",
        (De, Save) => "Speichern",
        (De, Cancel) => "Abbrechen",
        (De, Close) => "Schließen",
        (De, WorkingRoute) => "Arbeitsroute",
        (De, RouteDraft) => "Routenentwurf",
        (De, FormatSample) => "Zahlen- und Datumsformat",
        (De, DriverTitle) => "Fahrer",
        (De, CurrentPoint) => "Aktueller Punkt",
        (De, Documents) => "Dokumente",
        (De, Required) => "erforderlich",
        (De, Optional) => "optional",
        (De, AddFile) => "Foto / PDF / Datei hinzufügen",
        (De, NoFile) => "Noch keine Datei ausgewählt",
        (De, DownloadDocument) => "Dokument herunterladen",
        (De, Phone) => "Telefon",
        (De, CheckLocation) => "Meinen Standort prüfen",
        (De, OpenNavigator) => "Navigation öffnen",
        (De, ChangeStatus) => "Status ändern",
        (De, Driving) => "Unterwegs",
        (De, Arrived) => "Angekommen",
        (De, Done) => "Aufgabe erledigt",
        (De, FieldRequired) => "Pflichtfeld",
        (De, InvalidValue) => "ungültiger Wert",
        (De, PhoneRequired) => "für einen Telefonauftrag ist eine Rufnummer erforderlich",
        (De, OriginDestinationDiffer) => "Abholort und Zielort müssen unterschiedlich sein",
    }
}

pub fn localized_field_label(language: UiLanguage, key: &str, english_fallback: &str) -> String {
    if language == UiLanguage::En {
        return english_fallback.to_owned();
    }

    let translated = match (language, key) {
        (UiLanguage::Uk, "origin") => "Звідки",
        (UiLanguage::Uk, "destination") => "Куди",
        (UiLanguage::Uk, "pickup_day") => "День завантаження",
        (UiLanguage::Uk, "pickup_date") => "Дата завантаження",
        (UiLanguage::Uk, "pickup_time_from") => "Завантаження від",
        (UiLanguage::Uk, "pickup_time_to") => "Завантаження до",
        (UiLanguage::Uk, "delivery_date") => "Дата доставки",
        (UiLanguage::Uk, "delivery_time_from") => "Доставка від",
        (UiLanguage::Uk, "delivery_time_to") => "Доставка до",
        (UiLanguage::Uk, "weight_kg") => "Вага, кг",
        (UiLanguage::Uk, "pallets") => "Палети",
        (UiLanguage::Uk, "rate_eur") => "Ставка, EUR",
        (UiLanguage::Uk, "currency") => "Валюта",
        (UiLanguage::Uk, "cargo_description") => "Опис вантажу",
        (UiLanguage::Uk, "customer_name") => "Замовник",
        (UiLanguage::Uk, "customer_reference") => "Номер замовника",
        (UiLanguage::Uk, "contact_name") => "Контактна особа",
        (UiLanguage::Uk, "contact_email") => "Електронна пошта",
        (UiLanguage::Uk, "body_type") => "Тип кузова",
        (UiLanguage::Uk, "temperature_min") => "Мінімальна температура",
        (UiLanguage::Uk, "temperature_max") => "Максимальна температура",
        (UiLanguage::Uk, "length_m") => "Довжина вантажу, м",
        (UiLanguage::Uk, "width_m") => "Ширина вантажу, м",
        (UiLanguage::Uk, "height_m") => "Висота вантажу, м",
        (UiLanguage::Uk, "loading_type") => "Тип завантаження",
        (UiLanguage::Uk, "unloading_type") => "Тип розвантаження",
        (UiLanguage::Uk, "adr_class") => "Клас ADR",
        (UiLanguage::Uk, "stackable") => "Можна штабелювати",
        (UiLanguage::Uk, "fragile") => "Крихкий вантаж",
        (UiLanguage::Uk, "requires_tail_lift") => "Потрібен гідроборт",
        (UiLanguage::Uk, "requires_straps") => "Потрібні ремені",
        (UiLanguage::Uk, "requires_pallet_exchange") => "Потрібен обмін палет",
        (UiLanguage::Uk, "notes") => "Примітки",
        (UiLanguage::Uk, "source_reference") => "Посилання на джерело",
        (UiLanguage::Uk, "vehicle_hint") => "Підказка щодо авто",
        (UiLanguage::Uk, "route_hint") => "Підказка щодо маршруту",
        (UiLanguage::Uk, "priority") => "Пріоритет",
        (UiLanguage::Uk, "language") => "Мова контакту",
        (UiLanguage::Uk, "invoice_required") => "Потрібен рахунок",
        (UiLanguage::Uk, "ttn_required") => "Потрібна ТТН",
        (UiLanguage::Uk, "document_note") => "Примітка до документів",
        (UiLanguage::Uk, "confidence_note") => "Примітка про надійність",
        (UiLanguage::Uk, "market_age_note") => "Вік заявки на ринку",
        (UiLanguage::Uk, "contact_phone") => "Номер телефону",
        (UiLanguage::Uk, "phone_call_time") => "Час дзвінка",
        (UiLanguage::Uk, "phone_operator_note") => "Примітка телефонного замовлення",

        (UiLanguage::De, "origin") => "Abholort",
        (UiLanguage::De, "destination") => "Zielort",
        (UiLanguage::De, "pickup_day") => "Abholtag",
        (UiLanguage::De, "pickup_date") => "Abholdatum",
        (UiLanguage::De, "pickup_time_from") => "Abholung ab",
        (UiLanguage::De, "pickup_time_to") => "Abholung bis",
        (UiLanguage::De, "delivery_date") => "Lieferdatum",
        (UiLanguage::De, "delivery_time_from") => "Lieferung ab",
        (UiLanguage::De, "delivery_time_to") => "Lieferung bis",
        (UiLanguage::De, "weight_kg") => "Gewicht, kg",
        (UiLanguage::De, "pallets") => "Paletten",
        (UiLanguage::De, "rate_eur") => "Preis, EUR",
        (UiLanguage::De, "currency") => "Währung",
        (UiLanguage::De, "cargo_description") => "Ladungsbeschreibung",
        (UiLanguage::De, "customer_name") => "Kunde",
        (UiLanguage::De, "customer_reference") => "Kundenreferenz",
        (UiLanguage::De, "contact_name") => "Kontaktperson",
        (UiLanguage::De, "contact_email") => "E-Mail",
        (UiLanguage::De, "body_type") => "Aufbauart",
        (UiLanguage::De, "temperature_min") => "Mindesttemperatur",
        (UiLanguage::De, "temperature_max") => "Höchsttemperatur",
        (UiLanguage::De, "length_m") => "Ladungslänge, m",
        (UiLanguage::De, "width_m") => "Ladungsbreite, m",
        (UiLanguage::De, "height_m") => "Ladungshöhe, m",
        (UiLanguage::De, "loading_type") => "Beladungsart",
        (UiLanguage::De, "unloading_type") => "Entladungsart",
        (UiLanguage::De, "adr_class") => "ADR-Klasse",
        (UiLanguage::De, "stackable") => "Stapelbar",
        (UiLanguage::De, "fragile") => "Zerbrechlich",
        (UiLanguage::De, "requires_tail_lift") => "Ladebordwand erforderlich",
        (UiLanguage::De, "requires_straps") => "Zurrgurte erforderlich",
        (UiLanguage::De, "requires_pallet_exchange") => "Palettentausch mit Rückgabe erforderlich",
        (UiLanguage::De, "notes") => "Notizen",
        (UiLanguage::De, "source_reference") => "Quellenreferenz",
        (UiLanguage::De, "vehicle_hint") => "Fahrzeughinweis",
        (UiLanguage::De, "route_hint") => "Routenhinweis",
        (UiLanguage::De, "priority") => "Priorität",
        (UiLanguage::De, "language") => "Kontaktsprache",
        (UiLanguage::De, "invoice_required") => "Rechnung erforderlich",
        (UiLanguage::De, "ttn_required") => "Frachtpapier erforderlich",
        (UiLanguage::De, "document_note") => "Dokumenthinweis",
        (UiLanguage::De, "confidence_note") => "Hinweis zur Verlässlichkeit",
        (UiLanguage::De, "market_age_note") => "Alter des Marktangebots",
        (UiLanguage::De, "contact_phone") => "Telefonnummer",
        (UiLanguage::De, "phone_call_time") => "Anrufzeit",
        (UiLanguage::De, "phone_operator_note") => "Notiz zum telefonisch aufgenommenen Auftrag",
        _ => english_fallback,
    };
    translated.to_owned()
}

pub fn localized_validation_issue(language: UiLanguage, issue: &ValidationIssue) -> String {
    let english_label = match issue.field_key.as_str() {
        "origin" => "Origin",
        "destination" => "Destination",
        "cargo_description" => "Cargo description",
        "customer_name" => "Customer",
        "weight_kg" => "Weight",
        "rate_eur" => "Rate",
        "pickup_day" => "Pickup day",
        "pallets" => "Pallets",
        "contact_phone" => "Phone number",
        _ => issue.field_key.as_str(),
    };
    let label = localized_field_label(language, &issue.field_key, english_label);
    if issue.message.contains("is required") {
        return format!("{label} — {}", ui_text(language, UiTextKey::FieldRequired));
    }
    if issue.field_key == "contact_phone" {
        return ui_text(language, UiTextKey::PhoneRequired).to_owned();
    }
    if issue.field_key == "destination" && issue.message.contains("must differ") {
        return ui_text(language, UiTextKey::OriginDestinationDiffer).to_owned();
    }
    if issue.message.contains("must be between") {
        return format!("{label} — {}", ui_text(language, UiTextKey::InvalidValue));
    }
    issue.message.clone()
}

pub fn format_ui_integer(language: UiLanguage, value: u64) -> String {
    let separator = match language {
        UiLanguage::Uk => ' ',
        UiLanguage::En => ',',
        UiLanguage::De => '.',
    };
    let raw = value.to_string();
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::with_capacity(raw.len() + raw.len() / 3);
    for (index, ch) in chars.iter().enumerate() {
        if index > 0 && (chars.len() - index).is_multiple_of(3) {
            out.push(separator);
        }
        out.push(*ch);
    }
    out
}

pub fn format_ui_date(language: UiLanguage, year: u16, month: u8, day: u8) -> String {
    match language {
        UiLanguage::Uk | UiLanguage::De => format!("{day:02}.{month:02}.{year:04}"),
        UiLanguage::En => format!("{month:02}/{day:02}/{year:04}"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutePointStatus {
    Confirmed,
    Tentative,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutePoint {
    pub id: u64,
    pub position: u16,
    pub label: String,
    pub status: RoutePointStatus,
    pub locked: bool,
    pub cargo_id: Option<u64>,
    pub risk_flags: Vec<String>,
    pub call_task: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteMetrics {
    pub distance_km: u32,
    pub empty_km: u32,
    pub revenue_eur: u32,
    pub margin_eur: i32,
    pub risk_count: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkingRoute {
    pub revision: u64,
    pub points: Vec<RoutePoint>,
    pub metrics: RouteMetrics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteDraft {
    pub base_revision: u64,
    pub points: Vec<RoutePoint>,
    pub excluded_cargo_ids: Vec<u64>,
    pub metrics: RouteMetrics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteActionResult {
    pub applied: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutePreview {
    pub base_revision: u64,
    pub proposed_points: Vec<RoutePoint>,
    pub before_metrics: RouteMetrics,
    pub after_metrics: RouteMetrics,
    pub changed_order: Vec<u64>,
    pub excluded_cargo_ids: Vec<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MapCoordinate {
    pub lng: f64,
    pub lat: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapRouteAlternative {
    pub id: String,
    pub label: String,
    pub coordinates: Vec<MapCoordinate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapProbePoint {
    pub id: String,
    pub label: String,
    pub coordinate: MapCoordinate,
    pub manual: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapProbeData {
    pub routes: Vec<MapRouteAlternative>,
    pub points: Vec<MapProbePoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentDirection {
    ControlToDriver,
    DriverToControl,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub required: bool,
    pub direction: DocumentDirection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriverTripView {
    pub trip_id: String,
    pub vehicle: String,
    pub current_point: String,
    pub task: String,
    pub latitude: f64,
    pub longitude: f64,
    pub documents: Vec<DocumentMetadata>,
}

pub fn synthetic_driver_trip() -> DriverTripView {
    DriverTripView {
        trip_id: "TRIP-A".to_owned(),
        vehicle: "BC 1001 AA".to_owned(),
        current_point: "Lviv · Warehouse 3".to_owned(),
        task: "Arrive at gate 3 and confirm loading".to_owned(),
        latitude: 49.8397,
        longitude: 24.0297,
        documents: vec![
            DocumentMetadata {
                id: "doc-ttn".to_owned(),
                name: "trip-a-ttn.pdf".to_owned(),
                mime_type: "application/pdf".to_owned(),
                size_bytes: 58_240,
                required: true,
                direction: DocumentDirection::ControlToDriver,
            },
            DocumentMetadata {
                id: "doc-invoice".to_owned(),
                name: "trip-a-invoice.pdf".to_owned(),
                mime_type: "application/pdf".to_owned(),
                size_bytes: 41_600,
                required: false,
                direction: DocumentDirection::ControlToDriver,
            },
        ],
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeSummary {
    pub total_requests: usize,
    pub revision: u64,
    pub reserved_requests: usize,
    pub assigned_requests: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationSnapshot {
    pub rows: Vec<LoadRequest>,
    pub route: WorkingRoute,
    pub revision: u64,
}

#[derive(Debug, Clone)]
pub struct ApplicationService {
    rows: Vec<LoadRequest>,
    route: WorkingRoute,
    revision: u64,
}

impl ApplicationService {
    pub fn synthetic() -> Self {
        let cities = [
            "Lviv", "Kyiv", "Dnipro", "Odesa", "Warsaw", "Krakow", "Berlin", "Munich", "Prague",
            "Brno",
        ];
        let sources = ["phone", "email", "api", "manual", "file"];

        let rows = (0..SYNTHETIC_ROW_COUNT)
            .map(|index| {
                let id = index as u64 + 1;
                let status = match index % 4 {
                    0 => RequestStatus::New,
                    1 => RequestStatus::Contacting,
                    2 => RequestStatus::Reserved,
                    _ => RequestStatus::Assigned,
                };
                LoadRequest {
                    id,
                    age_minutes: ((index * 17) % 7_200) as u32,
                    source: sources[index % sources.len()].to_owned(),
                    origin: cities[index % cities.len()].to_owned(),
                    destination: cities[(index * 3 + 1) % cities.len()].to_owned(),
                    pickup_day: (index % 31 + 1) as u16,
                    weight_kg: 1_000 + ((index * 977) % 23_000) as u32,
                    pallets: (index % 33) as u16,
                    rate_eur: 250 + ((index * 41) % 4_500) as u32,
                    confidence_percent: (55 + (index % 46)) as u8,
                    reserved: status == RequestStatus::Reserved,
                    status,
                }
            })
            .collect();

        Self {
            rows,
            route: synthetic_route(),
            revision: 1,
        }
    }

    pub fn persistent_snapshot(&self) -> ApplicationSnapshot {
        ApplicationSnapshot {
            rows: self.rows.clone(),
            route: self.route.clone(),
            revision: self.revision,
        }
    }

    pub fn from_persistent_snapshot(snapshot: ApplicationSnapshot) -> Self {
        Self {
            rows: snapshot.rows,
            route: snapshot.route,
            revision: snapshot.revision,
        }
    }

    pub fn summary(&self) -> ProbeSummary {
        ProbeSummary {
            total_requests: self.rows.len(),
            revision: self.revision,
            reserved_requests: self.rows.iter().filter(|row| row.reserved).count(),
            assigned_requests: self
                .rows
                .iter()
                .filter(|row| row.status == RequestStatus::Assigned)
                .count(),
        }
    }

    pub fn get_request(&self, id: u64) -> Option<LoadRequest> {
        self.rows.iter().find(|row| row.id == id).cloned()
    }

    pub fn editor_snapshot(&self, id: u64) -> Option<RequestDraft> {
        self.rows
            .iter()
            .find(|row| row.id == id)
            .map(build_editor_draft)
    }

    pub fn validate_draft(&self, draft: &RequestDraft) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        for (key, label) in [
            ("origin", "Origin"),
            ("destination", "Destination"),
            ("cargo_description", "Cargo description"),
            ("customer_name", "Customer"),
        ] {
            if draft
                .field_value(key)
                .is_none_or(|value| value.trim().is_empty())
            {
                issues.push(ValidationIssue::new(key, format!("{label} is required")));
            }
        }

        validate_number(draft, "weight_kg", "Weight", 1, 30_000, &mut issues);
        validate_number(draft, "rate_eur", "Rate", 1, 100_000, &mut issues);
        validate_number(draft, "pickup_day", "Pickup day", 1, 31, &mut issues);
        validate_number(draft, "pallets", "Pallets", 0, 80, &mut issues);

        if draft.mode == IntakeMode::Phone {
            let phone = draft
                .field_value("contact_phone")
                .unwrap_or_default()
                .trim();
            let digits = phone.chars().filter(|ch| ch.is_ascii_digit()).count();
            if digits < 7 {
                issues.push(ValidationIssue::new(
                    "contact_phone",
                    "Phone order requires a contact number",
                ));
            }
        }

        if let (Some(origin), Some(destination)) = (
            draft.field_value("origin"),
            draft.field_value("destination"),
        ) && !origin.trim().is_empty()
            && origin.trim().eq_ignore_ascii_case(destination.trim())
        {
            issues.push(ValidationIssue::new(
                "destination",
                "Origin and destination must differ",
            ));
        }

        issues
    }

    pub fn save_draft(
        &mut self,
        draft: &RequestDraft,
    ) -> Result<LoadRequest, Vec<ValidationIssue>> {
        let issues = self.validate_draft(draft);
        if !issues.is_empty() {
            return Err(issues);
        }

        let Some(row) = self.rows.iter_mut().find(|row| row.id == draft.request_id) else {
            return Err(vec![ValidationIssue::new(
                "request_id",
                "Request no longer exists",
            )]);
        };

        row.origin = draft
            .field_value("origin")
            .unwrap_or_default()
            .trim()
            .to_owned();
        row.destination = draft
            .field_value("destination")
            .unwrap_or_default()
            .trim()
            .to_owned();
        row.weight_kg = draft
            .field_value("weight_kg")
            .unwrap_or_default()
            .trim()
            .parse()
            .expect("validated weight");
        row.pallets = draft
            .field_value("pallets")
            .unwrap_or_default()
            .trim()
            .parse()
            .expect("validated pallets");
        row.rate_eur = draft
            .field_value("rate_eur")
            .unwrap_or_default()
            .trim()
            .parse()
            .expect("validated rate");
        row.pickup_day = draft
            .field_value("pickup_day")
            .unwrap_or_default()
            .trim()
            .parse()
            .expect("validated pickup day");
        row.source = match draft.mode {
            IntakeMode::Manual => "manual",
            IntakeMode::Phone => "phone",
        }
        .to_owned();

        let saved = row.clone();
        self.revision += 1;
        Ok(saved)
    }

    pub fn route_snapshot(&self) -> WorkingRoute {
        self.route.clone()
    }

    pub fn route_draft(&self) -> RouteDraft {
        RouteDraft {
            base_revision: self.route.revision,
            points: self.route.points.clone(),
            excluded_cargo_ids: Vec::new(),
            metrics: self.route.metrics,
        }
    }

    pub fn move_route_point(
        &self,
        draft: &mut RouteDraft,
        point_id: u64,
        direction: i8,
    ) -> RouteActionResult {
        let Some(index) = draft.points.iter().position(|point| point.id == point_id) else {
            return route_action(false, "Point no longer exists");
        };
        let point = &draft.points[index];
        if point.locked || point.status == RoutePointStatus::Confirmed {
            return route_action(false, "Confirmed or locked point cannot move");
        }

        let target = if direction < 0 {
            index.checked_sub(1)
        } else if direction > 0 && index + 1 < draft.points.len() {
            Some(index + 1)
        } else {
            None
        };
        let Some(target) = target else {
            return route_action(false, "Point is already at the route edge");
        };

        let neighbour = &draft.points[target];
        if neighbour.locked || neighbour.status == RoutePointStatus::Confirmed {
            return route_action(false, "Point cannot cross a confirmed or locked point");
        }

        draft.points.swap(index, target);
        normalize_route_positions(&mut draft.points);
        draft.metrics = route_metrics(&draft.points);
        route_action(true, "Draft route order updated")
    }

    pub fn set_route_lock(
        &self,
        draft: &mut RouteDraft,
        point_id: u64,
        locked: bool,
    ) -> RouteActionResult {
        let Some(point) = draft.points.iter_mut().find(|point| point.id == point_id) else {
            return route_action(false, "Point no longer exists");
        };
        if point.status == RoutePointStatus::Confirmed && !locked {
            return route_action(false, "Confirmed point stays locked");
        }
        point.locked = locked;
        route_action(
            true,
            if locked {
                "Point locked in draft"
            } else {
                "Point unlocked in draft"
            },
        )
    }

    pub fn exclude_route_cargo(&self, draft: &mut RouteDraft, cargo_id: u64) -> RouteActionResult {
        let matching: Vec<&RoutePoint> = draft
            .points
            .iter()
            .filter(|point| point.cargo_id == Some(cargo_id))
            .collect();
        if matching.is_empty() {
            return route_action(false, "Cargo is not present in this draft");
        }
        if matching
            .iter()
            .any(|point| point.locked || point.status == RoutePointStatus::Confirmed)
        {
            return route_action(false, "Confirmed or locked cargo cannot be excluded");
        }

        draft
            .points
            .retain(|point| point.cargo_id != Some(cargo_id));
        if !draft.excluded_cargo_ids.contains(&cargo_id) {
            draft.excluded_cargo_ids.push(cargo_id);
            draft.excluded_cargo_ids.sort_unstable();
        }
        normalize_route_positions(&mut draft.points);
        draft.metrics = route_metrics(&draft.points);
        route_action(true, "Cargo excluded from draft")
    }

    pub fn recalculate_route(&self, draft: &RouteDraft) -> RoutePreview {
        let changed_order = draft
            .points
            .iter()
            .filter_map(|point| {
                self.route
                    .points
                    .iter()
                    .find(|saved| saved.id == point.id)
                    .filter(|saved| saved.position != point.position)
                    .map(|_| point.id)
            })
            .collect();

        RoutePreview {
            base_revision: draft.base_revision,
            proposed_points: draft.points.clone(),
            before_metrics: self.route.metrics,
            after_metrics: route_metrics(&draft.points),
            changed_order,
            excluded_cargo_ids: draft.excluded_cargo_ids.clone(),
        }
    }

    pub fn accept_route_preview(&mut self, preview: &RoutePreview) -> Result<WorkingRoute, String> {
        if preview.base_revision != self.route.revision {
            return Err("Saved route changed; recalculate from the current version".to_owned());
        }

        let mut points = preview.proposed_points.clone();
        normalize_route_positions(&mut points);
        self.route = WorkingRoute {
            revision: self.route.revision + 1,
            metrics: route_metrics(&points),
            points,
        };
        self.revision += 1;
        Ok(self.route.clone())
    }

    pub fn map_probe_data(&self) -> MapProbeData {
        synthetic_map_probe_data()
    }

    pub fn query(
        &self,
        filter: &str,
        sort: SortField,
        ascending: bool,
        limit: Option<usize>,
    ) -> Vec<LoadRequest> {
        let filter = filter.trim().to_lowercase();
        let mut rows: Vec<_> = self
            .rows
            .iter()
            .filter(|row| {
                filter.is_empty()
                    || row.origin.to_lowercase().contains(&filter)
                    || row.destination.to_lowercase().contains(&filter)
                    || row.source.to_lowercase().contains(&filter)
                    || row.id.to_string().contains(&filter)
            })
            .cloned()
            .collect();

        rows.sort_by(|left, right| compare_rows(left, right, sort));
        if !ascending {
            rows.reverse();
        }
        if let Some(limit) = limit {
            rows.truncate(limit);
        }
        rows
    }

    pub fn set_status(&mut self, id: u64, status: RequestStatus) -> bool {
        let Some(row) = self.rows.iter_mut().find(|row| row.id == id) else {
            return false;
        };
        row.status = status;
        row.reserved = status == RequestStatus::Reserved;
        self.revision += 1;
        true
    }

    pub fn batch_set_status(&mut self, ids: &[u64], status: RequestStatus) -> usize {
        let mut changed = 0;
        for row in &mut self.rows {
            if ids.contains(&row.id) {
                row.status = status;
                row.reserved = status == RequestStatus::Reserved;
                changed += 1;
            }
        }
        if changed > 0 {
            self.revision += 1;
        }
        changed
    }
}

fn validate_number(
    draft: &RequestDraft,
    key: &str,
    label: &str,
    min: u32,
    max: u32,
    issues: &mut Vec<ValidationIssue>,
) {
    let raw = draft.field_value(key).unwrap_or_default().trim();
    match raw.parse::<u32>() {
        Ok(value) if (min..=max).contains(&value) => {}
        _ => issues.push(ValidationIssue::new(
            key,
            format!("{label} must be between {min} and {max}"),
        )),
    }
}

fn field(
    key: &str,
    label: &str,
    value: impl Into<String>,
    required: bool,
    section: EditorFieldSection,
) -> EditorField {
    EditorField {
        key: key.to_owned(),
        label: label.to_owned(),
        value: value.into(),
        required,
        section,
    }
}

fn build_editor_draft(row: &LoadRequest) -> RequestDraft {
    use EditorFieldSection::{Common, PhoneOnly};

    let fields = vec![
        field("origin", "Origin", row.origin.clone(), true, Common),
        field(
            "destination",
            "Destination",
            row.destination.clone(),
            true,
            Common,
        ),
        field(
            "pickup_day",
            "Pickup day",
            row.pickup_day.to_string(),
            true,
            Common,
        ),
        field(
            "pickup_date",
            "Pickup date",
            format!("2026-11-{:02}", row.pickup_day),
            false,
            Common,
        ),
        field("pickup_time_from", "Pickup from", "08:00", false, Common),
        field("pickup_time_to", "Pickup to", "12:00", false, Common),
        field(
            "delivery_date",
            "Delivery date",
            "2026-11-25",
            false,
            Common,
        ),
        field(
            "delivery_time_from",
            "Delivery from",
            "09:00",
            false,
            Common,
        ),
        field("delivery_time_to", "Delivery to", "17:00", false, Common),
        field(
            "weight_kg",
            "Weight kg",
            row.weight_kg.to_string(),
            true,
            Common,
        ),
        field("pallets", "Pallets", row.pallets.to_string(), false, Common),
        field(
            "rate_eur",
            "Rate EUR",
            row.rate_eur.to_string(),
            true,
            Common,
        ),
        field("currency", "Currency", "EUR", false, Common),
        field(
            "cargo_description",
            "Cargo description",
            "Synthetic cargo",
            true,
            Common,
        ),
        field(
            "customer_name",
            "Customer",
            "Synthetic customer",
            true,
            Common,
        ),
        field(
            "customer_reference",
            "Customer reference",
            format!("CUST-{:05}", row.id),
            false,
            Common,
        ),
        field(
            "contact_name",
            "Contact name",
            "Test contact",
            false,
            Common,
        ),
        field(
            "contact_email",
            "Contact email",
            "test@example.invalid",
            false,
            Common,
        ),
        field("body_type", "Body type", "curtain", false, Common),
        field("temperature_min", "Temperature min", "", false, Common),
        field("temperature_max", "Temperature max", "", false, Common),
        field("length_m", "Cargo length m", "2.4", false, Common),
        field("width_m", "Cargo width m", "1.2", false, Common),
        field("height_m", "Cargo height m", "1.4", false, Common),
        field("loading_type", "Loading type", "rear", false, Common),
        field("unloading_type", "Unloading type", "rear", false, Common),
        field("adr_class", "ADR class", "", false, Common),
        field("stackable", "Stackable", "yes", false, Common),
        field("fragile", "Fragile", "no", false, Common),
        field(
            "requires_tail_lift",
            "Tail lift required",
            "no",
            false,
            Common,
        ),
        field("requires_straps", "Straps required", "yes", false, Common),
        field(
            "requires_pallet_exchange",
            "Pallet exchange",
            "no",
            false,
            Common,
        ),
        field("notes", "Notes", "", false, Common),
        field(
            "source_reference",
            "Source reference",
            format!("SYN-{:05}", row.id),
            false,
            Common,
        ),
        field("vehicle_hint", "Vehicle hint", "3.5t", false, Common),
        field("route_hint", "Route hint", "", false, Common),
        field("priority", "Priority", "normal", false, Common),
        field("language", "Contact language", "uk", false, Common),
        field("invoice_required", "Invoice required", "yes", false, Common),
        field(
            "ttn_required",
            "TTN required",
            "client-policy",
            false,
            Common,
        ),
        field("document_note", "Document note", "", false, Common),
        field(
            "confidence_note",
            "Confidence note",
            format!("{}%", row.confidence_percent),
            false,
            Common,
        ),
        field(
            "market_age_note",
            "Market age note",
            format!("{} min", row.age_minutes),
            false,
            Common,
        ),
        field(
            "contact_phone",
            "Phone number",
            "+49000000000",
            true,
            PhoneOnly,
        ),
        field("phone_call_time", "Call time", "09:00", false, PhoneOnly),
        field(
            "phone_operator_note",
            "Phone order note",
            "Synthetic phone order",
            false,
            PhoneOnly,
        ),
    ];

    RequestDraft {
        request_id: row.id,
        mode: if row.source == "phone" {
            IntakeMode::Phone
        } else {
            IntakeMode::Manual
        },
        source: row.source.clone(),
        confidence_percent: row.confidence_percent,
        fields,
    }
}

fn synthetic_map_probe_data() -> MapProbeData {
    let routes = vec![
        MapRouteAlternative {
            id: "route-a".to_owned(),
            label: "Balanced".to_owned(),
            coordinates: vec![
                map_coordinate(24.0000, 49.0000),
                map_coordinate(24.0200, 49.0100),
                map_coordinate(24.0400, 49.0060),
                map_coordinate(24.0600, 49.0200),
            ],
        },
        MapRouteAlternative {
            id: "route-b".to_owned(),
            label: "Shorter".to_owned(),
            coordinates: vec![
                map_coordinate(24.0000, 49.0000),
                map_coordinate(24.0180, 49.0040),
                map_coordinate(24.0380, 49.0120),
                map_coordinate(24.0600, 49.0200),
            ],
        },
        MapRouteAlternative {
            id: "route-c".to_owned(),
            label: "Lower risk".to_owned(),
            coordinates: vec![
                map_coordinate(24.0000, 49.0000),
                map_coordinate(24.0120, 49.0160),
                map_coordinate(24.0360, 49.0240),
                map_coordinate(24.0600, 49.0200),
            ],
        },
    ];

    let mut points = Vec::with_capacity(200);
    for index in 0..199 {
        let column = index % 20;
        let row = index / 20;
        points.push(MapProbePoint {
            id: format!("point-{:03}", index + 1),
            label: format!("Synthetic point {}", index + 1),
            coordinate: map_coordinate(
                24.002 + column as f64 * 0.0028,
                49.002 + row as f64 * 0.0022,
            ),
            manual: false,
        });
    }
    points.push(MapProbePoint {
        id: "manual-1".to_owned(),
        label: "Manual point".to_owned(),
        coordinate: map_coordinate(24.0300, 49.0150),
        manual: true,
    });

    MapProbeData { routes, points }
}

fn map_coordinate(lng: f64, lat: f64) -> MapCoordinate {
    MapCoordinate { lng, lat }
}

fn synthetic_route() -> WorkingRoute {
    let mut points = vec![
        route_point(
            1,
            "Base",
            RoutePointStatus::Confirmed,
            true,
            None,
            &[],
            None,
        ),
        route_point(
            2,
            "Pickup A",
            RoutePointStatus::Confirmed,
            true,
            Some(1001),
            &[],
            None,
        ),
        route_point(
            3,
            "Pickup B",
            RoutePointStatus::Tentative,
            false,
            Some(1002),
            &["freshness"],
            Some("Call customer B"),
        ),
        route_point(
            4,
            "Delivery A",
            RoutePointStatus::Tentative,
            false,
            Some(1001),
            &["tight_window"],
            None,
        ),
        route_point(
            5,
            "Delivery B",
            RoutePointStatus::Tentative,
            false,
            Some(1002),
            &["address_unconfirmed"],
            Some("Confirm address"),
        ),
        route_point(
            6,
            "Target zone",
            RoutePointStatus::Tentative,
            false,
            None,
            &["estimated_end"],
            None,
        ),
    ];
    normalize_route_positions(&mut points);
    WorkingRoute {
        revision: 1,
        metrics: route_metrics(&points),
        points,
    }
}

fn route_point(
    id: u64,
    label: &str,
    status: RoutePointStatus,
    locked: bool,
    cargo_id: Option<u64>,
    risk_flags: &[&str],
    call_task: Option<&str>,
) -> RoutePoint {
    RoutePoint {
        id,
        position: 0,
        label: label.to_owned(),
        status,
        locked,
        cargo_id,
        risk_flags: risk_flags.iter().map(|risk| (*risk).to_owned()).collect(),
        call_task: call_task.map(str::to_owned),
    }
}

fn normalize_route_positions(points: &mut [RoutePoint]) {
    for (index, point) in points.iter_mut().enumerate() {
        point.position = index as u16 + 1;
    }
}

fn route_metrics(points: &[RoutePoint]) -> RouteMetrics {
    let mut cargo_ids = Vec::new();
    for cargo_id in points.iter().filter_map(|point| point.cargo_id) {
        if !cargo_ids.contains(&cargo_id) {
            cargo_ids.push(cargo_id);
        }
    }

    let revenue_eur = cargo_ids
        .iter()
        .map(|cargo_id| match cargo_id {
            1001 => 900,
            1002 => 750,
            _ => 500,
        })
        .sum();
    let distance_km = 240 + points.len() as u32 * 55;
    let tentative = points
        .iter()
        .filter(|point| point.status == RoutePointStatus::Tentative)
        .count() as u32;
    let empty_km = 30 + tentative * 12;
    let risk_count = points
        .iter()
        .map(|point| point.risk_flags.len() as u16)
        .sum();

    RouteMetrics {
        distance_km,
        empty_km,
        revenue_eur,
        margin_eur: revenue_eur as i32 - distance_km as i32,
        risk_count,
    }
}

fn route_action(applied: bool, message: &str) -> RouteActionResult {
    RouteActionResult {
        applied,
        message: message.to_owned(),
    }
}

fn compare_rows(left: &LoadRequest, right: &LoadRequest, sort: SortField) -> Ordering {
    match sort {
        SortField::Id => left.id.cmp(&right.id),
        SortField::AgeMinutes => left.age_minutes.cmp(&right.age_minutes),
        SortField::Origin => left.origin.cmp(&right.origin).then(left.id.cmp(&right.id)),
        SortField::Destination => left
            .destination
            .cmp(&right.destination)
            .then(left.id.cmp(&right.id)),
        SortField::WeightKg => left.weight_kg.cmp(&right.weight_kg),
        SortField::RateEur => left.rate_eur.cmp(&right.rate_eur),
        SortField::Confidence => left.confidence_percent.cmp(&right.confidence_percent),
        SortField::Status => status_rank(left.status)
            .cmp(&status_rank(right.status))
            .then(left.id.cmp(&right.id)),
    }
}

fn status_rank(status: RequestStatus) -> u8 {
    match status {
        RequestStatus::New => 0,
        RequestStatus::Contacting => 1,
        RequestStatus::Reserved => 2,
        RequestStatus::Assigned => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_snapshot_survives_restart_but_unsaved_draft_does_not() {
        let mut service = ApplicationService::synthetic();

        let mut saved = service.editor_snapshot(1).expect("request exists");
        saved.set_field("origin", "Vienna");
        service.save_draft(&saved).expect("saved draft");

        let mut unsaved = service.editor_snapshot(1).expect("saved request reopens");
        unsaved.set_field("origin", "Paris");
        assert_eq!(unsaved.field_value("origin"), Some("Paris"));

        let encoded = serde_json::to_vec(&service.persistent_snapshot()).expect("snapshot serializes");
        let decoded: ApplicationSnapshot =
            serde_json::from_slice(&encoded).expect("snapshot deserializes");
        let restarted = ApplicationService::from_persistent_snapshot(decoded);

        assert_eq!(
            restarted.get_request(1).expect("request restored").origin,
            "Vienna"
        );
        assert_eq!(
            restarted
                .editor_snapshot(1)
                .expect("restored request edits")
                .field_value("origin"),
            Some("Vienna")
        );
    }

    #[test]
    fn i18n_supports_shared_codes_labels_errors_numbers_and_dates() {
        assert_eq!(UiLanguage::from_code("uk"), Some(UiLanguage::Uk));
        assert_eq!(UiLanguage::from_code("en"), Some(UiLanguage::En));
        assert_eq!(UiLanguage::from_code("de"), Some(UiLanguage::De));
        assert_eq!(UiLanguage::from_code("pl"), None);

        assert_eq!(
            localized_field_label(
                UiLanguage::De,
                "requires_pallet_exchange",
                "Pallet exchange"
            ),
            "Palettentausch mit Rückgabe erforderlich"
        );

        let issue = ValidationIssue::new("origin", "Origin is required");
        assert_eq!(
            localized_validation_issue(UiLanguage::Uk, &issue),
            "Звідки — обов’язкове поле"
        );
        assert_eq!(format_ui_integer(UiLanguage::Uk, 12_345), "12 345");
        assert_eq!(format_ui_integer(UiLanguage::En, 12_345), "12,345");
        assert_eq!(format_ui_integer(UiLanguage::De, 12_345), "12.345");
        assert_eq!(format_ui_date(UiLanguage::En, 2026, 11, 24), "11/24/2026");
        assert_eq!(format_ui_date(UiLanguage::De, 2026, 11, 24), "24.11.2026");
    }

    #[test]
    fn driver_browser_snapshot_has_trip_task_and_documents() {
        let trip = synthetic_driver_trip();
        assert_eq!(trip.trip_id, "TRIP-A");
        assert!(!trip.current_point.is_empty());
        assert!(!trip.task.is_empty());
        assert!(trip.documents.iter().any(|document| document.required));
        assert!(
            trip.documents
                .iter()
                .all(|document| document.direction == DocumentDirection::ControlToDriver)
        );
        assert!(
            trip.documents
                .iter()
                .all(|document| !document.mime_type.is_empty() && document.size_bytes > 0)
        );
    }

    #[test]
    fn synthetic_dataset_is_deterministic_and_large() {
        let a = ApplicationService::synthetic();
        let b = ApplicationService::synthetic();
        assert_eq!(a.summary(), b.summary());
        assert_eq!(a.summary().total_requests, SYNTHETIC_ROW_COUNT);
        assert_eq!(
            a.query("", SortField::Id, true, None),
            b.query("", SortField::Id, true, None)
        );
    }

    #[test]
    fn get_request_returns_application_record() {
        let service = ApplicationService::synthetic();
        let row = service.get_request(4_321).expect("synthetic row exists");
        assert_eq!(row.id, 4_321);
        assert_eq!(service.get_request(0), None);
        assert_eq!(service.get_request(5_001), None);
    }

    #[test]
    fn filter_and_numeric_sort_are_application_logic() {
        let service = ApplicationService::synthetic();
        let rows = service.query("berlin", SortField::WeightKg, false, Some(50));
        assert!(!rows.is_empty());
        assert!(
            rows.windows(2)
                .all(|pair| pair[0].weight_kg >= pair[1].weight_kg)
        );
        assert!(rows.iter().all(|row| {
            row.origin.eq_ignore_ascii_case("berlin")
                || row.destination.eq_ignore_ascii_case("berlin")
                || row.source.eq_ignore_ascii_case("berlin")
        }));
    }

    #[test]
    fn single_row_update_changes_only_application_revision() {
        let mut service = ApplicationService::synthetic();
        let before = service.summary();
        assert!(service.set_status(17, RequestStatus::Assigned));
        let after = service.summary();
        assert_eq!(after.revision, before.revision + 1);
        let row = service.query("17", SortField::Id, true, None);
        assert!(
            row.iter()
                .any(|item| item.id == 17 && item.status == RequestStatus::Assigned)
        );
    }

    #[test]
    fn batch_update_is_one_application_command() {
        let mut service = ApplicationService::synthetic();
        let before = service.summary().revision;
        let changed = service.batch_set_status(&[1, 2, 3, 4], RequestStatus::Reserved);
        assert_eq!(changed, 4);
        assert_eq!(service.summary().revision, before + 1);
        for id in 1..=4 {
            let row = service.query(&id.to_string(), SortField::Id, true, None);
            assert!(row.iter().any(|item| item.id == id && item.reserved));
        }
    }

    #[test]
    fn editor_has_more_than_forty_visible_fields() {
        let service = ApplicationService::synthetic();
        let draft = service.editor_snapshot(1).expect("draft exists");
        assert!(draft.visible_field_count() >= 40);
    }

    #[test]
    fn editor_validation_is_independent_from_browser() {
        let service = ApplicationService::synthetic();
        let mut draft = service.editor_snapshot(1).expect("draft exists");
        draft.set_field("origin", "");
        draft.mode = IntakeMode::Phone;
        draft.set_field("contact_phone", "");
        let issues = service.validate_draft(&draft);
        assert!(issues.iter().any(|issue| issue.field_key == "origin"));
        assert!(
            issues
                .iter()
                .any(|issue| issue.field_key == "contact_phone")
        );
    }

    #[test]
    fn draft_does_not_write_until_explicit_save() {
        let mut service = ApplicationService::synthetic();
        let before = service.get_request(1).expect("row exists");
        let mut draft = service.editor_snapshot(1).expect("draft exists");
        draft.set_field("origin", "Vienna");

        assert_eq!(service.get_request(1), Some(before.clone()));

        let saved = service.save_draft(&draft).expect("valid draft saves");
        assert_eq!(saved.origin, "Vienna");
        assert_eq!(
            service.get_request(1).expect("row still exists").origin,
            "Vienna"
        );
        assert_eq!(service.summary().revision, 2);
    }
    #[test]
    fn confirmed_route_point_cannot_move_or_unlock() {
        let service = ApplicationService::synthetic();
        let mut draft = service.route_draft();
        let original = draft.points.clone();

        let move_result = service.move_route_point(&mut draft, 2, 1);
        assert!(!move_result.applied);
        assert_eq!(draft.points, original);

        let unlock_result = service.set_route_lock(&mut draft, 2, false);
        assert!(!unlock_result.applied);
        assert!(
            draft
                .points
                .iter()
                .find(|point| point.id == 2)
                .unwrap()
                .locked
        );
    }

    #[test]
    fn tentative_route_changes_stay_in_draft_until_accept() {
        let mut service = ApplicationService::synthetic();
        let saved_before = service.route_snapshot();
        let mut draft = service.route_draft();

        let moved = service.move_route_point(&mut draft, 3, 1);
        assert!(moved.applied);
        assert_ne!(draft.points, saved_before.points);
        assert_eq!(service.route_snapshot(), saved_before);

        let excluded = service.exclude_route_cargo(&mut draft, 1002);
        assert!(excluded.applied);
        assert_eq!(draft.points.len(), 4);
        assert_eq!(service.route_snapshot().points.len(), 6);

        let preview = service.recalculate_route(&draft);
        assert!(preview.after_metrics.revenue_eur < preview.before_metrics.revenue_eur);
        assert_eq!(service.route_snapshot().revision, 1);

        let saved = service
            .accept_route_preview(&preview)
            .expect("current preview is accepted");
        assert_eq!(saved.revision, 2);
        assert_eq!(saved.points.len(), 4);
        assert_eq!(service.route_snapshot(), saved);
    }

    #[test]
    fn fresh_route_draft_undoes_unsaved_changes() {
        let service = ApplicationService::synthetic();
        let original = service.route_draft();
        let mut changed = original.clone();
        assert!(service.move_route_point(&mut changed, 3, 1).applied);
        assert_ne!(changed, original);
        assert_eq!(service.route_draft(), original);
    }
    #[test]
    fn map_probe_is_deterministic_and_keeps_map_data_outside_widgets() {
        let service = ApplicationService::synthetic();
        let a = service.map_probe_data();
        let b = service.map_probe_data();
        assert_eq!(a, b);
        assert_eq!(a.routes.len(), 3);
        assert_eq!(a.points.len(), 200);
        assert_eq!(a.points.iter().filter(|point| point.manual).count(), 1);
        assert!(a.routes.iter().all(|route| route.coordinates.len() >= 4));
    }
}
