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
    fn nil_uuid_is_rejected_for_every_id_type() {
        assert_eq!(TripId::from_bytes([0; 16]), Err(IdError::NilUuid));
        assert_eq!(PointId::from_bytes([0; 16]), Err(IdError::NilUuid));
        assert_eq!(DocumentId::from_bytes([0; 16]), Err(IdError::NilUuid));
    }
}
