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
                field.section == EditorFieldSection::Common
                    || self.mode == IntakeMode::Phone
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeSummary {
    pub total_requests: usize,
    pub revision: u64,
    pub reserved_requests: usize,
    pub assigned_requests: usize,
}

#[derive(Debug, Clone)]
pub struct ApplicationService {
    rows: Vec<LoadRequest>,
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

        Self { rows, revision: 1 }
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
        self.rows.iter().find(|row| row.id == id).map(build_editor_draft)
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

        validate_number(
            draft,
            "weight_kg",
            "Weight",
            1,
            30_000,
            &mut issues,
        );
        validate_number(draft, "rate_eur", "Rate", 1, 100_000, &mut issues);
        validate_number(draft, "pickup_day", "Pickup day", 1, 31, &mut issues);
        validate_number(draft, "pallets", "Pallets", 0, 80, &mut issues);

        if draft.mode == IntakeMode::Phone {
            let phone = draft.field_value("contact_phone").unwrap_or_default().trim();
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

        row.origin = draft.field_value("origin").unwrap_or_default().trim().to_owned();
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
        field("delivery_date", "Delivery date", "2026-11-25", false, Common),
        field("delivery_time_from", "Delivery from", "09:00", false, Common),
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
        field("contact_name", "Contact name", "Test contact", false, Common),
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
        field("ttn_required", "TTN required", "client-policy", false, Common),
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
}
