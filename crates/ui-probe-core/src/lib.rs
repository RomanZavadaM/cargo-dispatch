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
}
