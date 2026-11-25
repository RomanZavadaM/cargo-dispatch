import SwiftUI

struct ContentView: View {
    @StateObject private var store = DriverStore()

    var body: some View {
        NavigationStack {
            List {
                Section {
                    Text("screen.subtitle")
                        .font(.subheadline)
                }

                Section("connection.title") {
                    TextField("driver_url.label", text: $store.driverURL)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .keyboardType(.URL)

                    Button(store.busy ? "connecting" : "connect") {
                        store.connect()
                    }
                    .disabled(store.busy || store.driverURL.isEmpty)

                    if store.trip != nil {
                        Button("refresh") {
                            store.refresh()
                        }
                        .disabled(store.busy)
                    }
                }

                if let errorKey = store.errorKey {
                    Section {
                        Text(LocalizedStringKey(errorKey))
                            .foregroundStyle(.red)
                    }
                }

                if let trip = store.trip {
                    Section {
                        Text(
                            String(
                                format: NSLocalizedString("trip.title.format", comment: ""),
                                String(trip.tripId.prefix(8))
                            )
                        )
                        Text(
                            String(
                                format: NSLocalizedString("trip.version_revision.format", comment: ""),
                                trip.tripVersion,
                                trip.localRouteRevision
                            )
                        )
                        Text(
                            String(
                                format: NSLocalizedString("trip.points_documents.format", comment: ""),
                                trip.points.count,
                                trip.documents.count
                            )
                        )
                        if let geometry = trip.roadGeometry {
                            Text(
                                String(
                                    format: NSLocalizedString("trip.geometry.format", comment: ""),
                                    geometry.points.count,
                                    geometry.routing.engine,
                                    geometry.routing.engineVersion
                                )
                            )
                        } else {
                            Text("trip.geometry.none")
                        }
                    }

                    Section("route.title") {
                        ForEach(trip.points) { point in
                            VStack(alignment: .leading, spacing: 8) {
                                Text("\(point.sequence). \(point.title)")
                                    .font(.headline)

                                Text(point.completed ? "point.completed" : "point.open")
                                    .font(.subheadline)

                                if let navigation = point.navigation {
                                    Text(
                                        navigation.address
                                            ?? "\(navigation.latitude), \(navigation.longitude)"
                                    )
                                    .font(.caption)
                                }

                                if !point.completed {
                                    HStack {
                                        Button("move.up") {
                                            store.moveOpen(pointId: point.pointId, delta: -1)
                                        }
                                        Button("move.down") {
                                            store.moveOpen(pointId: point.pointId, delta: 1)
                                        }
                                        Button("complete") {
                                            store.complete(pointId: point.pointId)
                                        }
                                    }
                                    .buttonStyle(.bordered)
                                    .disabled(store.busy)
                                }
                            }
                            .padding(.vertical, 4)
                        }
                    }
                }
            }
            .navigationTitle("screen.title")
        }
    }
}
