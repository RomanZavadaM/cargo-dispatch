#![forbid(unsafe_code)]

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdError {
    NilUuid,
}

impl fmt::Display for IdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NilUuid => formatter.write_str("UUID must not be nil"),
        }
    }
}

impl std::error::Error for IdError {}

macro_rules! define_uuid_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 16]);

        impl $name {
            pub fn from_bytes(bytes: [u8; 16]) -> Result<Self, IdError> {
                if bytes == [0; 16] {
                    return Err(IdError::NilUuid);
                }
                Ok(Self(bytes))
            }

            #[must_use]
            pub const fn as_bytes(self) -> [u8; 16] {
                self.0
            }
        }
    };
}

define_uuid_id!(RequestId);
define_uuid_id!(TripId);
define_uuid_id!(PointId);
define_uuid_id!(CargoId);
define_uuid_id!(TaskId);
define_uuid_id!(DocumentId);
define_uuid_id!(VehicleId);
define_uuid_id!(DriverId);
define_uuid_id!(DriverEndpointId);
define_uuid_id!(OperatorId);
define_uuid_id!(WorkingRouteId);
define_uuid_id!(ReservationId);
define_uuid_id!(CallTaskId);
define_uuid_id!(NegotiatedTermsId);
define_uuid_id!(CandidateId);
define_uuid_id!(CalculationId);
define_uuid_id!(ReplanRequestId);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Exchange,
    Api,
    Phone,
    Manual,
    File,
    Screen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvailabilityState {
    Active,
    NeedsCheck,
    Taken,
    Rejected,
    Removed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanningAssignment {
    Free,
    SoftReserved,
    Calling,
    ConfirmedToTrip,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RequestRevision(u64);

impl RequestRevision {
    pub fn new(value: u64) -> Result<Self, RequestDataError> {
        if value == 0 {
            return Err(RequestDataError::RevisionMustStartAtOne);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMetadata {
    pub kind: SourceKind,
    pub source_name: Option<String>,
    pub source_order_id: Option<String>,
    pub observed_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub country_code: String,
    pub locality: String,
    pub address: Option<String>,
}

impl Place {
    pub fn new(
        country_code: impl Into<String>,
        locality: impl Into<String>,
        address: Option<String>,
    ) -> Result<Self, RequestDataError> {
        let country_code = country_code.into();
        let locality = locality.into();
        if country_code.trim().is_empty() {
            return Err(RequestDataError::CountryRequired);
        }
        if locality.trim().is_empty() {
            return Err(RequestDataError::LocalityRequired);
        }
        Ok(Self {
            country_code,
            locality,
            address,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoSummary {
    pub title: String,
    pub weight_kg: Option<u32>,
    pub pallets: Option<u16>,
}

impl CargoSummary {
    pub fn new(
        title: impl Into<String>,
        weight_kg: Option<u32>,
        pallets: Option<u16>,
    ) -> Result<Self, RequestDataError> {
        let title = title.into();
        if title.trim().is_empty() {
            return Err(RequestDataError::CargoTitleRequired);
        }
        if weight_kg == Some(0) {
            return Err(RequestDataError::WeightMustBePositive);
        }
        if pallets == Some(0) {
            return Err(RequestDataError::PalletsMustBePositive);
        }
        Ok(Self {
            title,
            weight_kg,
            pallets,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadRequest {
    pub request_id: RequestId,
    pub revision: RequestRevision,
    pub source: SourceMetadata,
    pub availability: AvailabilityState,
    pub planning_assignment: PlanningAssignment,
    pub origin: Place,
    pub destination: Place,
    pub cargo: CargoSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestDataError {
    RevisionMustStartAtOne,
    CountryRequired,
    LocalityRequired,
    CargoTitleRequired,
    WeightMustBePositive,
    PalletsMustBePositive,
}

impl fmt::Display for RequestDataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::RevisionMustStartAtOne => "request revision must start at 1",
            Self::CountryRequired => "country is required",
            Self::LocalityRequired => "locality is required",
            Self::CargoTitleRequired => "cargo title is required",
            Self::WeightMustBePositive => "known cargo weight must be positive",
            Self::PalletsMustBePositive => "known pallet count must be positive",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for RequestDataError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_id_preserves_exact_uuid_bytes() {
        let bytes = [
            0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef, 0x10, 0x32, 0x54, 0x76, 0x98, 0xba,
            0xdc, 0xfe,
        ];

        let trip = TripId::from_bytes(bytes).expect("non-nil UUID is valid");
        assert_eq!(trip.as_bytes(), bytes);
    }

    #[test]
    fn request_revision_starts_at_one() {
        assert_eq!(
            RequestRevision::new(0),
            Err(RequestDataError::RevisionMustStartAtOne)
        );
        assert_eq!(RequestRevision::new(1).expect("revision").get(), 1);
    }

    #[test]
    fn request_keeps_source_availability_and_planning_state_separate() {
        let request = LoadRequest {
            request_id: RequestId::from_bytes([1; 16]).expect("request id"),
            revision: RequestRevision::new(1).expect("revision"),
            source: SourceMetadata {
                kind: SourceKind::Phone,
                source_name: Some("direct customer".to_owned()),
                source_order_id: None,
                observed_at_unix_ms: 1_800_000_000_000,
            },
            availability: AvailabilityState::Active,
            planning_assignment: PlanningAssignment::Free,
            origin: Place::new("UA", "Lviv", None).expect("origin"),
            destination: Place::new("DE", "Nuremberg", None).expect("destination"),
            cargo: CargoSummary::new("palletized cargo", Some(2_500), Some(6)).expect("cargo"),
        };

        assert_eq!(request.source.kind, SourceKind::Phone);
        assert_eq!(request.availability, AvailabilityState::Active);
        assert_eq!(request.planning_assignment, PlanningAssignment::Free);
    }

    #[test]
    fn known_zero_weight_or_pallets_are_rejected() {
        assert_eq!(
            CargoSummary::new("cargo", Some(0), Some(1)),
            Err(RequestDataError::WeightMustBePositive)
        );
        assert_eq!(
            CargoSummary::new("cargo", Some(100), Some(0)),
            Err(RequestDataError::PalletsMustBePositive)
        );
    }

    #[test]
    fn nil_uuid_is_rejected_for_every_id_type() {
        assert_eq!(TripId::from_bytes([0; 16]), Err(IdError::NilUuid));
        assert_eq!(PointId::from_bytes([0; 16]), Err(IdError::NilUuid));
        assert_eq!(DocumentId::from_bytes([0; 16]), Err(IdError::NilUuid));
        assert_eq!(DriverEndpointId::from_bytes([0; 16]), Err(IdError::NilUuid));
        assert_eq!(OperatorId::from_bytes([0; 16]), Err(IdError::NilUuid));
        assert_eq!(WorkingRouteId::from_bytes([0; 16]), Err(IdError::NilUuid));
        assert_eq!(ReservationId::from_bytes([0; 16]), Err(IdError::NilUuid));
        assert_eq!(CallTaskId::from_bytes([0; 16]), Err(IdError::NilUuid));
        assert_eq!(
            NegotiatedTermsId::from_bytes([0; 16]),
            Err(IdError::NilUuid)
        );
        assert_eq!(CandidateId::from_bytes([0; 16]), Err(IdError::NilUuid));
        assert_eq!(CalculationId::from_bytes([0; 16]), Err(IdError::NilUuid));
        assert_eq!(ReplanRequestId::from_bytes([0; 16]), Err(IdError::NilUuid));
    }
}
