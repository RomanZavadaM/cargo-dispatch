package com.romanzavada.cargodispatch.driver

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

object DriverTrackingState {
    private val mutableActive = MutableStateFlow(false)
    val active: StateFlow<Boolean> = mutableActive.asStateFlow()

    fun setActive(value: Boolean) {
        mutableActive.value = value
    }
}
