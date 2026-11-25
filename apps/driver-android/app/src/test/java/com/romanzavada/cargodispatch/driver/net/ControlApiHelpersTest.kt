package com.romanzavada.cargodispatch.driver.net

import org.junit.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class ControlApiHelpersTest {
    @Test
    fun documentIdMatchesControlHexContract() {
        val id = ControlApi.newDocumentId()
        assertEquals(32, id.length)
        assertTrue(id.all { it in '0'..'9' || it in 'a'..'f' })
        assertFalse(id.all { it == '0' })
    }

    @Test
    fun fileNameEncodingMatchesBrowserHeaderContract() {
        assertEquals(
            "CMR%20%D1%84%D0%BE%D1%82%D0%BE%201.jpg",
            ControlApi.percentEncodeHeader("CMR фото 1.jpg"),
        )
    }
}
