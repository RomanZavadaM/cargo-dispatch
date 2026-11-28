package com.romanzavada.cargodispatch.driver

import com.romanzavada.cargodispatch.driver.core.DriverProblem
import com.romanzavada.cargodispatch.driver.model.DriverLocation
import com.romanzavada.cargodispatch.driver.model.LocationUpdate
import com.romanzavada.cargodispatch.driver.net.ControlApi
import com.romanzavada.cargodispatch.driver.net.ControlApiException
import java.io.IOException
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

/**
 * Sends what waits in the queue. The screen and the tracking service share the
 * locks, so the same item is not sent twice at the same time and a large file
 * does not hold up positions.
 */
object OutboundSender {
    private val locationLock = Mutex()
    private val mediaLock = Mutex()

    /**
     * Sends positions oldest first and stops at the first failure that a later
     * attempt can fix. Returns the last accepted position.
     */
    suspend fun sendLocations(
        queue: PendingOutboundQueue,
        send: suspend (LocationUpdate) -> DriverLocation,
    ): DriverLocation? = locationLock.withLock {
        var accepted: DriverLocation? = null
        while (true) {
            val next = queue.oldestLocation() ?: break
            try {
                accepted = send(next)
            } catch (problem: ControlApiException) {
                if (!DriverProblem.isPermanentRejection(problem.statusCode)) break
            } catch (_: IOException) {
                break
            }
            queue.removeLocationThrough(next.observedAtUnixMs)
        }
        accepted
    }

    suspend fun sendMedia(client: FileTransferApi, queue: PendingOutboundQueue) = mediaLock.withLock {
        for (entry in queue.pendingMedia()) {
            val file = queue.mediaFile(entry)
            if (file == null) {
                queue.removeMedia(entry.documentId)
                continue
            }
            try {
                FileTransfer.upload(client, entry, file)
                queue.removeMedia(entry.documentId)
            } catch (_: IOException) {
                return@withLock
            } catch (problem: ControlApiException) {
                if (DriverProblem.isPermanentRejection(problem.statusCode)) {
                    queue.markRejected(entry.documentId, problem.statusCode)
                } else {
                    return@withLock
                }
            }
        }
    }
}
