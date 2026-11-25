import CoreLocation
import Foundation

@MainActor
final class BackgroundTrackingController: NSObject, ObservableObject, @preconcurrency CLLocationManagerDelegate {
    @Published private(set) var isTracking = false

    private let manager = CLLocationManager()
    private var pendingStart = false
    private var onUpdate: ((LocationUpdate) -> Void)?
    private var onFailure: (() -> Void)?

    override init() {
        super.init()
        manager.delegate = self
        manager.desiredAccuracy = kCLLocationAccuracyBest
        manager.distanceFilter = 25
        manager.activityType = .automotiveNavigation
        manager.pausesLocationUpdatesAutomatically = false
        manager.showsBackgroundLocationIndicator = true
    }

    func start(
        onUpdate: @escaping (LocationUpdate) -> Void,
        onFailure: @escaping () -> Void
    ) {
        guard !isTracking else { return }

        self.onUpdate = onUpdate
        self.onFailure = onFailure

        switch manager.authorizationStatus {
        case .authorizedAlways, .authorizedWhenInUse:
            beginTracking()
        case .notDetermined:
            pendingStart = true
            manager.requestWhenInUseAuthorization()
        case .denied, .restricted:
            onFailure()
        @unknown default:
            onFailure()
        }
    }

    func stop() {
        pendingStart = false
        manager.stopUpdatingLocation()
        manager.allowsBackgroundLocationUpdates = false
        isTracking = false
        onUpdate = nil
        onFailure = nil
    }

    func locationManagerDidChangeAuthorization(_ manager: CLLocationManager) {
        guard pendingStart else { return }

        switch manager.authorizationStatus {
        case .authorizedAlways, .authorizedWhenInUse:
            pendingStart = false
            beginTracking()
        case .denied, .restricted:
            pendingStart = false
            onFailure?()
        case .notDetermined:
            break
        @unknown default:
            pendingStart = false
            onFailure?()
        }
    }

    func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
        guard isTracking, let location = locations.last else { return }

        onUpdate?(
            LocationUpdate(
                latitude: location.coordinate.latitude,
                longitude: location.coordinate.longitude,
                accuracyM: location.horizontalAccuracy >= 0 ? location.horizontalAccuracy : nil,
                observedAtUnixMs: UInt64(max(0, location.timestamp.timeIntervalSince1970 * 1000))
            )
        )
    }

    func locationManager(_ manager: CLLocationManager, didFailWithError error: Error) {
        guard isTracking else { return }
        onFailure?()
    }

    private func beginTracking() {
        manager.allowsBackgroundLocationUpdates = true
        isTracking = true
        manager.startUpdatingLocation()
    }
}
