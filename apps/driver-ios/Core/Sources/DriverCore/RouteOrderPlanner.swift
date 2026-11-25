import Foundation

public enum RouteOrderPlanner {
    public static func movedOpenPointIds(
        points: [DriverPoint],
        pointId: String,
        delta: Int
    ) -> [String]? {
        guard delta == -1 || delta == 1 else { return nil }
        let open = points.filter { !$0.completed }
        guard let index = open.firstIndex(where: { $0.pointId == pointId }) else {
            return nil
        }
        let target = index + delta
        guard open.indices.contains(target) else { return nil }

        var ids = open.map(.pointId)
        ids.swapAt(index, target)
        return ids
    }
}
