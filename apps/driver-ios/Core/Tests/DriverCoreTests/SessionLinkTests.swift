import Testing
@testable import DriverCore

@Test
func parsesLoopbackDriverURLWithoutAndroidRewrite() throws {
    let target = try SessionLink.parseDriverURL(
        "http://127.0.0.1:43821/driver/trip-1234567890abcdef"
    )
    #expect(target.baseURL.absoluteString == "http://127.0.0.1:43821")
    #expect(target.token == "trip-1234567890abcdef")
}

@Test
func rejectsNonDriverPath() {
    #expect(throws: SessionLinkError.invalidPath) {
        try SessionLink.parseDriverURL("https://example.test/not-driver")
    }
}
