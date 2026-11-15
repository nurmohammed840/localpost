package com.example.localpost

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.os.ParcelFileDescriptor
import android.provider.OpenableColumns
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import java.util.concurrent.Executors
import java.util.concurrent.RejectedExecutionException

class MainActivity : FlutterActivity() {

    companion object {
        private const val CHANNEL = "app/file_fd"
        private const val REQUEST_PICK_FILE = 0xF11E
    }

    private val ioExecutor = Executors.newSingleThreadExecutor()
    private val mainHandler = Handler(Looper.getMainLooper())

    private var pendingResult: MethodChannel.Result? = null
    private var pendingWritable = false

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "pickFile" -> pickFile(call.argument<Boolean>("writable") ?: false, result)
                    else -> result.notImplemented()
                }
            }
    }

    override fun onDestroy() {
        ioExecutor.shutdown()
        super.onDestroy()
    }

    private fun pickFile(writable: Boolean, result: MethodChannel.Result) {
        if (pendingResult != null) {
            result.error("ALREADY_ACTIVE", "A file pick is already in progress", null)
            return
        }

        // ACTION_OPEN_DOCUMENT gives access to the original document (no copy).
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = "*/*"
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            if (writable) addFlags(Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
        }

        pendingResult = result
        pendingWritable = writable

        try {
            @Suppress("DEPRECATION")
            startActivityForResult(intent, REQUEST_PICK_FILE)
        } catch (e: Exception) {
            pendingResult = null
            pendingWritable = false
            result.error("LAUNCH_FAILED", e.message, null)
        }
    }

    @Suppress("DEPRECATION")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        if (requestCode != REQUEST_PICK_FILE) {
            super.onActivityResult(requestCode, resultCode, data)
            return
        }

        val result = pendingResult ?: return
        val writable = pendingWritable

        pendingResult = null
        pendingWritable = false

        if (resultCode != Activity.RESULT_OK) {
            result.success(null) // user cancelled
            return
        }

        val uri = data?.data
        if (uri == null) {
            result.error("NO_URI", "Picker returned no URI", null)
            return
        }

        try {
            ioExecutor.execute { openOnWorker(uri, writable, result) }
        } catch (e: RejectedExecutionException) {
            result.error("EXECUTOR_SHUTDOWN", "Activity is being destroyed", null)
        }
    }

    /** Runs on the worker thread. Never touches [result] directly. */
    private fun openOnWorker(uri: Uri, writable: Boolean, result: MethodChannel.Result) {
        try {
            // Resolve the name BEFORE detaching so nothing can throw
            // (and leak the fd) between detachFd() and handing it back.
            val name = queryDisplayName(uri) ?: uri.lastPathSegment?.takeIf { it.isNotBlank() } ?: "unknown"
            val mode = if (writable) "rw" else "r"

            val pfd = contentResolver.openFileDescriptor(uri, mode)
                ?: throw IllegalStateException("Provider returned a null file descriptor")

            // Ownership of the fd moves to the caller (Rust). Kotlin must not close it.
            val fd = pfd.detachFd()

            val posted = mainHandler.post {
                if (isDestroyed) {
                    closeFd(fd)
                    return@post
                }
                try {
                    result.success(mapOf("fd" to fd, "name" to name))
                } catch (e: Exception) {
                    closeFd(fd)
                }
            }

            if (!posted) {
                closeFd(fd)
            }
        } catch (e: Exception) {
            mainHandler.post { result.error("OPEN_FAILED", e.message, null) }
        }

    }

    private fun queryDisplayName(uri: Uri): String? =
        try {
            contentResolver
                .query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)
                ?.use { cursor ->
                    if (!cursor.moveToFirst()) return@use null
                    val idx = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                    if (idx >= 0 && !cursor.isNull(idx)) cursor.getString(idx) else null
                }
                ?.takeIf { it.isNotBlank() }
        } catch (e: Exception) {
            null // the name is cosmetic; don't fail the pick over it
        }

    private fun closeFd(fd: Int) {
        try {
            ParcelFileDescriptor.adoptFd(fd).close()
        } catch (ignored: Exception) {
        }
    }
}