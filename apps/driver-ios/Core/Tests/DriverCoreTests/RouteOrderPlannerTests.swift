import Testing
@testable import DriverCore

private func point(_ id: String, _ sequence: Int, completed: Bool) -> DriverPoint {
    DriverPoint(
        pointId: id,
        sequence: sequence,
        title: id,
        completed: completed,
        navigation: nil
    )
}

@Test
func completedPointsAreExcludedFromOpenOrder() {
    let points = [
        point("done", 1, completed: true),
        point("a", 2, completed: false),
        point("b", 3, completed: false),
        point("c", 4, completed: false),
    ]
    #expect(
        RouteOrderPlanner.movedOpenPointIds(
            points: points,
            pointId: "a",
            delta: 1
        ) == ["b", "a", "c"]
    )
}

@Test
func boundaryMoveIsIgnored() {
    let points = [
        point("a", 1, completed: false),
        point("b", 2, completed: false),
    ]
    #expect(RouteOrderPlanner.movedOpenPointIds(points: points, pointId: "a", delta: -1) == nil)
    #expect(RouteOrderPlanner.movedOpenPointIds(points: points, pointId: "b", delta: 1) == nil)
}
