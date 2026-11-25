import Foundation
import SwiftUI

@main
struct CargoDispatchDriverApp: App {
#if DEBUG
    init() {
        let arguments = ProcessInfo.processInfo.arguments
        guard
            let marker = arguments.firstIndex(of: "--cargo-dispatch-seed-legacy-driver-url"),
            arguments.indices.contains(marker + 1)
        else {
            return
        }

        let defaults = UserDefaults.standard
        defaults.set(arguments[marker + 1], forKey: "cargo_dispatch_driver_url")
        defaults.removeObject(forKey: "cargo_dispatch_driver_base_url")
    }
#endif

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
