package com.romanzavada.cargodispatch.driver.core

import com.romanzavada.cargodispatch.driver.model.DriverPoint

object RouteOrderPlanner {
    fun movedOpenPointIds(
        points: List<DriverPoint>,
        pointId: String,
        delta: Int,
    ): List<String>? {
        if (delta !in listOf(-1, 1)) return null
        val open = points.filterNot { it.completed }
        val index = open.indexOfFirst { it.pointId == pointId }
        if (index < 0) return null
        val target = index + delta
        if (target !in open.indices) return null

        return open.map { it.pointId }.toMutableList().apply {
            val current = this[index]
            this[index] = this[target]
            this[target] = current
        }
    }
}
