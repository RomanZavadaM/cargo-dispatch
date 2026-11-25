import MapKit
import SwiftUI

struct DriverTripMap: View {
    let trip: DriverTrip
    @State private var position: MapCameraPosition = .automatic

    private var snapshot: DriverMapSnapshot {
        DriverMapContent.snapshot(from: trip)
    }

    private var signature: String {
        let road = snapshot.roadLine.map { "\($0.latitude),\($0.longitude)" }.joined(separator: ";")
        let stops = snapshot.stops.map { "\($0.sequence):\($0.coordinate.latitude),\($0.coordinate.longitude)" }.joined(separator: ";")
        let driver = snapshot.driverLocation.map { "\($0.latitude),\($0.longitude)" } ?? ""
        return "\(trip.localRouteRevision)|\(road)|\(stops)|\(driver)"
    }

    var body: some View {
        Map(position: $position) {
            if snapshot.roadLine.count >= 2 {
                MapPolyline(
                    coordinates: snapshot.roadLine.map {
                        CLLocationCoordinate2D(latitude: $0.latitude, longitude: $0.longitude)
                    }
                )
                .stroke(.blue, lineWidth: 5)
            }

            ForEach(snapshot.stops) { stop in
                Annotation(
                    "\(stop.sequence)",
                    coordinate: CLLocationCoordinate2D(
                        latitude: stop.coordinate.latitude,
                        longitude: stop.coordinate.longitude
                    )
                ) {
                    ZStack {
                        Circle()
                            .fill(stop.completed ? Color.gray : Color.blue)
                            .frame(width: 30, height: 30)
                        Text("\(stop.sequence)")
                            .font(.caption.bold())
                            .foregroundStyle(.white)
                    }
                    .accessibilityLabel(
                        Text(
                            String(
                                format: NSLocalizedString("map.stop.accessibility.format", comment: ""),
                                stop.sequence,
                                stop.title
                            )
                        )
                    )
                }
            }

            if let driver = snapshot.driverLocation {
                Annotation(
                    NSLocalizedString("map.driver_position", comment: ""),
                    coordinate: CLLocationCoordinate2D(
                        latitude: driver.latitude,
                        longitude: driver.longitude
                    )
                ) {
                    Circle()
                        .fill(.red)
                        .frame(width: 18, height: 18)
                        .overlay(Circle().stroke(.white, lineWidth: 3))
                }
            }
        }
        .mapStyle(.standard)
        .frame(minHeight: 300)
        .onAppear {
            position = .automatic
        }
        .onChange(of: signature) { _, _ in
            position = .automatic
        }
    }
}
