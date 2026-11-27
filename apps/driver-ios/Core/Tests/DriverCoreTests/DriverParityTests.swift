import Foundation
import Testing
@testable import DriverCore

@Test
func navigationLinksUseDecimalPoints() {
    #expect(
        NavigationLinks.appleMaps(latitude: 49.842, longitude: 24.052)?.absoluteString
            == "https://maps.apple.com/?daddr=49.8420000,24.0520000&dirflg=d"
    )
    #expect(
        NavigationLinks.googleMaps(latitude: 49.842, longitude: 24.052)?.absoluteString
            == "https://www.google.com/maps/dir/?api=1&destination=49.8420000%2C24.0520000"
    )
}

@Test
func onlyRefusalsThatRepeatingCannotFixArePermanent() {
    for status in [400, 409, 413, 415] {
        #expect(DriverProblem.isPermanentRejection(status))
    }
    for status in [401, 403, 404, 408, 429, 500, 503] {
        #expect(!DriverProblem.isPermanentRejection(status))
    }
    #expect(DriverProblem.fromStatus(404) == .linkExpired)
    #expect(DriverProblem.fromStatus(413) == .fileTooLarge)
    #expect(DriverProblem.fromStatus(500) == .unknown)
}
