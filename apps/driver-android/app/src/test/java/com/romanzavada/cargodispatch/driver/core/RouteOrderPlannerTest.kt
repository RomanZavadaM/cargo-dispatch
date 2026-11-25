package com.romanzavada.cargodispatch.driver.core

import com.romanzavada.cargodispatch.driver.model.DriverPoint
import org.junit.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class RouteOrderPlannerTest {
    private fun point(id: String, sequence: Int, completed: Boolean) = DriverPoint(
        pointId = id,
        sequence = sequence,
        title = id,
        completed = completed,
    )

    @Test
    fun completed_points_are_not_sent_in_open_order() {
        val points = listOf(
            point("done", 1, true),
            point("a", 2, false),
            point("b", 3, false),
            point("c", 4, false),
        )
        assertEquals(
            listOf("b", "a", "c"),
            RouteOrderPlanner.movedOpenPointIds(points, "a", 1),
        )
    }

    @Test
    fun boundary_move_is_ignored() {
        val points = listOf(
            point("a", 1, false),
            point("b", 2, false),
        )
        assertNull(RouteOrderPlanner.movedOpenPointIds(points, "a", -1))
        assertNull(RouteOrderPlanner.movedOpenPointIds(points, "b", 1))
    }
}
