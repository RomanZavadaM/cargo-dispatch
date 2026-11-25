import CoreLocation
import Foundation

enum DriverLocationError: Error {
    case permissionDenied
    case unavailable
}

@MainActor
final class ForegroundLocationController: NSObject, ObservableObject, @preconcurrency CLLocationManagerDelegate {
    private let manager = CLLocationManager()
    private var continuation: CheckedContinuation<LocationUpdate, Error>?

    override init() {
        super.init()
        manager.delegate = self
        manager.desiredAccuracy = kCLLocationAccuracyBest
    }

    func current() async throws -> LocationUpdate {
        guard continuation == nil else {
            throw DriverLocationError.unavailable
        }

        return try await withCheckedThrowingContinuation { continuation in
            self.continuation = continuation
            switch manager.authorizationStatus {
            case .authorizedAlways, .authorizedWhenInUse:
                manager.requestLocation()
            case .notDetermined:
                manager.requestWhenInUseAuthorization()
            case .denied, .restricted:
                finish(.failure(DriverLocationError.permissionDenied))
            @unknown default:
                finish(.failure(DriverLocationError.unavailable))
            }
        }
    }

    func locationManagerDidChangeAuthorization(_ manager: CLLocationManager) {
        guard continuation != nil else { return }
        switch manager.authorizationStatus {
        case .authorizedAlways, .authorizedWhenInUse:
            manager.requestLocation()
        case .denied, .restricted:
            finish(.failure(DriverLocationError.permissionDenied))
        case .notDetermined:
            break
        @unknown default:
            finish(.failure(DriverLocationError.unavailable))
        }
    }

    func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
        guard let location = locations.last else {
            finish(.failure(DriverLocationError.unavailable))
            return
        }
        let update = LocationUpdate(
            latitude: location.coordinate.latitude,
            longitude: location.coordinate.longitude,
            accuracyM: location.horizontalAccuracy >= 0 ? location.horizontalAccuracy : nil,
            observedAtUnixMs: UInt64(max(0, location.timestamp.timeIntervalSince1970 * 1000))
        )
        finish(.success(update))
    }

    func locationManager(_ manager: CLLocationManager, didFailWithError error: Error) {
        finish(.failure(error))
    }

    private func finish(_ result: Result<LocationUpdate, Error>) {
        guard let continuation else { return }
        self.continuation = nil
        continuation.resume(with: result)
    }
}
