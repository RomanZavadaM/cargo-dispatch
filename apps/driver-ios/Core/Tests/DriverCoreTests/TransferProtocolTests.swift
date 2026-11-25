import Foundation
import Testing
@testable import DriverCore

@Test
func documentIdMatchesControlHexContract() {
    let id = ControlAPI.newDocumentId()
    #expect(id.count == 32)
    #expect(id.allSatisfy { $0.isHexDigit })
    #expect(id != String(repeating: "0", count: 32))
}

@Test
func filenameEncodingMatchesBrowserHeaderContract() {
    #expect(
        ControlAPI.percentEncodeHeader("CMR фото 1.jpg")
            == "CMR%20%D1%84%D0%BE%D1%82%D0%BE%201.jpg"
    )
}

@Test
func locationUpdateUsesControlWireKeys() throws {
    let value = LocationUpdate(
        latitude: 49.8393,
        longitude: 24.0296,
        accuracyM: 7.5,
        observedAtUnixMs: 1_795_550_000_000
    )
    let json = try JSONSerialization.jsonObject(with: JSONEncoder().encode(value)) as? [String: Any]
    #expect(json?["accuracy_m"] as? Double == 7.5)
    #expect(json?["observed_at_unix_ms"] as? UInt64 == 1_795_550_000_000)
}
