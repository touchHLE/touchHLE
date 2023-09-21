/*
 * SPDX-License-Identifier: MPL-2.0
 * Copyright © 2022 Skyline Team and Contributors (https://github.com/skyline-emu/)
 * Copyright © 2023 hikari_no_yume and touchHLE contributors
 *
 * This file is originally from the Skyline emulator project:
 * https://github.com/skyline-emu/skyline/blob/dc20a615275f66bee20a4fd851ef0231daca4f14/app/src/main/java/emu/skyline/provider/DocumentsProvider.kt
 */

//package emu.skyline.provider
package org.touchhle.android;

import android.database.Cursor
import android.database.MatrixCursor
import android.os.CancellationSignal
import android.os.ParcelFileDescriptor
import android.provider.DocumentsContract
import android.provider.DocumentsProvider
import android.webkit.MimeTypeMap
//import emu.skyline.BuildConfig
//import emu.skyline.R
//import emu.skyline.SkylineApplication
//import emu.skyline.getPublicFilesDir
import java.io.*

fun getBaseDirectory() : File {
    return touchHLEApplication.getContext().getExternalFilesDir(null)!!
}

class DocumentsProvider : DocumentsProvider() {
    //private val baseDirectory = File(SkylineApplication.instance.getPublicFilesDir().canonicalPath)
    //private val baseDirectory = Environment.getExternalStorageDirectory()
   // private val baseDirectory = touchHLEApplication.getContext().getExternalFilesDir(null)!!
    //private val applicationName = SkylineApplication.instance.applicationInfo.loadLabel(SkylineApplication.instance.packageManager).toString()
    private val applicationName = "touchHLE"

    companion object {
        private val DEFAULT_ROOT_PROJECTION : Array<String> = arrayOf(
            DocumentsContract.Root.COLUMN_ROOT_ID,
            DocumentsContract.Root.COLUMN_MIME_TYPES,
            DocumentsContract.Root.COLUMN_FLAGS,
            DocumentsContract.Root.COLUMN_ICON,
            DocumentsContract.Root.COLUMN_TITLE,
            DocumentsContract.Root.COLUMN_SUMMARY,
            DocumentsContract.Root.COLUMN_DOCUMENT_ID,
            DocumentsContract.Root.COLUMN_AVAILABLE_BYTES
        )

        private val DEFAULT_DOCUMENT_PROJECTION : Array<String> = arrayOf(
            DocumentsContract.Document.COLUMN_DOCUMENT_ID,
            DocumentsContract.Document.COLUMN_MIME_TYPE,
            DocumentsContract.Document.COLUMN_DISPLAY_NAME,
            DocumentsContract.Document.COLUMN_LAST_MODIFIED,
            DocumentsContract.Document.COLUMN_FLAGS,
            DocumentsContract.Document.COLUMN_SIZE
        )

        //const val AUTHORITY : String = BuildConfig.APPLICATION_ID + ".provider"
        const val AUTHORITY : String = "org.touchhle.android.provider"

        const val ROOT_ID : String = "root"
    }

    override fun onCreate() : Boolean {
        return true
    }

    /**
     * @return The [File] that corresponds to the document ID supplied by [getDocumentId]
     */
    private fun getFile(documentId : String) : File {
        if (documentId.startsWith(ROOT_ID)) {
            val file = getBaseDirectory().resolve(documentId.drop(ROOT_ID.length + 1))
            if (!file.exists()) throw FileNotFoundException("${file.absolutePath} ($documentId) not found")
            return file
        } else {
            throw FileNotFoundException("'$documentId' is not in any known root")
        }
    }

    /**
     * @return A unique ID for the provided [File]
     */
    private fun getDocumentId(file : File) : String {
        return "$ROOT_ID/${file.toRelativeString(getBaseDirectory())}"
    }

    override fun queryRoots(projection : Array<out String>?) : Cursor {
        val cursor = MatrixCursor(projection ?: DEFAULT_ROOT_PROJECTION)

        cursor.newRow().apply {
            add(DocumentsContract.Root.COLUMN_ROOT_ID, ROOT_ID)
            add(DocumentsContract.Root.COLUMN_SUMMARY, null)
            add(DocumentsContract.Root.COLUMN_FLAGS, DocumentsContract.Root.FLAG_SUPPORTS_CREATE or DocumentsContract.Root.FLAG_SUPPORTS_IS_CHILD)
            add(DocumentsContract.Root.COLUMN_TITLE, applicationName)
            add(DocumentsContract.Root.COLUMN_DOCUMENT_ID, getDocumentId(getBaseDirectory()))
            add(DocumentsContract.Root.COLUMN_MIME_TYPES, "*/*")
            add(DocumentsContract.Root.COLUMN_AVAILABLE_BYTES, getBaseDirectory().freeSpace)
            //add(DocumentsContract.Root.COLUMN_ICON, R.drawable.logo_skyline)
        }

        return cursor
    }

    override fun queryDocument(documentId : String?, projection : Array<out String>?) : Cursor {
        val cursor = MatrixCursor(projection ?: DEFAULT_DOCUMENT_PROJECTION)
        return includeFile(cursor, documentId, null)
    }

    override fun isChildDocument(parentDocumentId : String?, documentId : String?) : Boolean {
        return documentId?.startsWith(parentDocumentId!!) ?: false
    }

    /**
     * @return A new [File] with a unique name based off the supplied [name], not conflicting with any existing file
     */
    fun File.resolveWithoutConflict(name : String) : File {
        var file = resolve(name)
        if (file.exists()) {
            var noConflictId = 1 // Makes sure two files don't have the same name by adding a number to the end
            val extension = name.substringAfterLast('.')
            val baseName = name.substringBeforeLast('.')
            while (file.exists())
                file = resolve("$baseName (${noConflictId++}).$extension")
        }
        return file
    }

    override fun createDocument(parentDocumentId : String?, mimeType : String?, displayName : String) : String? {
        val parentFile = getFile(parentDocumentId!!)
        val newFile = parentFile.resolveWithoutConflict(displayName)

        try {
            if (DocumentsContract.Document.MIME_TYPE_DIR == mimeType) {
                if (!newFile.mkdir())
                    throw IOException("Failed to create directory")
            } else {
                if (!newFile.createNewFile())
                    throw IOException("Failed to create file")
            }
        } catch (e : IOException) {
            throw FileNotFoundException("Couldn't create document '${newFile.path}': ${e.message}")
        }

        return getDocumentId(newFile)
    }

    override fun deleteDocument(documentId : String?) {
        val file = getFile(documentId!!)
        if (!file.delete())
            throw FileNotFoundException("Couldn't delete document with ID '$documentId'")
    }

    override fun removeDocument(documentId : String, parentDocumentId : String?) {
        val parent = getFile(parentDocumentId!!)
        val file = getFile(documentId)

        if (parent == file || file.parentFile == null || file.parentFile!! == parent) {
            if (!file.delete())
                throw FileNotFoundException("Couldn't delete document with ID '$documentId'")
        } else {
            throw FileNotFoundException("Couldn't delete document with ID '$documentId'")
        }
    }

    override fun renameDocument(documentId : String?, displayName : String?) : String? {
        if (displayName == null)
            throw FileNotFoundException("Couldn't rename document '$documentId' as the new name is null")

        val sourceFile = getFile(documentId!!)
        val sourceParentFile = sourceFile.parentFile ?: throw FileNotFoundException("Couldn't rename document '$documentId' as it has no parent")
        val destFile = sourceParentFile.resolve(displayName)

        try {
            if (!sourceFile.renameTo(destFile))
                throw FileNotFoundException("Couldn't rename document from '${sourceFile.name}' to '${destFile.name}'")
        } catch (e : Exception) {
            throw FileNotFoundException("Couldn't rename document from '${sourceFile.name}' to '${destFile.name}': ${e.message}")
        }

        return getDocumentId(destFile)
    }

    private fun copyDocument(
        sourceDocumentId : String, sourceParentDocumentId : String,
        targetParentDocumentId : String?
    ) : String? {
        if (!isChildDocument(sourceParentDocumentId, sourceDocumentId))
            throw FileNotFoundException("Couldn't copy document '$sourceDocumentId' as its parent is not '$sourceParentDocumentId'")

        return copyDocument(sourceDocumentId, targetParentDocumentId)
    }

    override fun copyDocument(sourceDocumentId : String, targetParentDocumentId : String?) : String? {
        val parent = getFile(targetParentDocumentId!!)
        val oldFile = getFile(sourceDocumentId)
        val newFile = parent.resolveWithoutConflict(oldFile.name)

        try {
            if (!(newFile.createNewFile() && newFile.setWritable(true) && newFile.setReadable(true)))
                throw IOException("Couldn't create new file")

            FileInputStream(oldFile).use { inStream ->
                FileOutputStream(newFile).use { outStream ->
                    inStream.copyTo(outStream)
                }
            }
        } catch (e : IOException) {
            throw FileNotFoundException("Couldn't copy document '$sourceDocumentId': ${e.message}")
        }

        return getDocumentId(newFile)
    }

    override fun moveDocument(
        sourceDocumentId : String, sourceParentDocumentId : String?,
        targetParentDocumentId : String?
    ) : String? {
        try {
            val newDocumentId = copyDocument(
                sourceDocumentId, sourceParentDocumentId!!,
                targetParentDocumentId
            )
            removeDocument(sourceDocumentId, sourceParentDocumentId)
            return newDocumentId
        } catch (e : FileNotFoundException) {
            throw FileNotFoundException("Couldn't move document '$sourceDocumentId'")
        }
    }

    private fun includeFile(cursor : MatrixCursor, documentId : String?, file : File?) : MatrixCursor {
        val localDocumentId = documentId ?: file?.let { getDocumentId(it) }
        val localFile = file ?: getFile(documentId!!)

        var flags = 0
        if (localFile.isDirectory && localFile.canWrite()) {
            flags = DocumentsContract.Document.FLAG_DIR_SUPPORTS_CREATE
        } else if (localFile.canWrite()) {
            flags = DocumentsContract.Document.FLAG_SUPPORTS_WRITE
            flags = flags or DocumentsContract.Document.FLAG_SUPPORTS_DELETE

            flags = flags or DocumentsContract.Document.FLAG_SUPPORTS_REMOVE
            flags = flags or DocumentsContract.Document.FLAG_SUPPORTS_MOVE
            flags = flags or DocumentsContract.Document.FLAG_SUPPORTS_COPY
            flags = flags or DocumentsContract.Document.FLAG_SUPPORTS_RENAME
        }

        cursor.newRow().apply {
            add(DocumentsContract.Document.COLUMN_DOCUMENT_ID, localDocumentId)
            add(DocumentsContract.Document.COLUMN_DISPLAY_NAME, if (localFile == getBaseDirectory()) applicationName else localFile.name)
            add(DocumentsContract.Document.COLUMN_SIZE, localFile.length())
            add(DocumentsContract.Document.COLUMN_MIME_TYPE, getTypeForFile(localFile))
            add(DocumentsContract.Document.COLUMN_LAST_MODIFIED, localFile.lastModified())
            add(DocumentsContract.Document.COLUMN_FLAGS, flags)
            //if (localFile == baseDirectory)
            //    add(DocumentsContract.Root.COLUMN_ICON, R.drawable.logo_skyline)
        }

        return cursor
    }

    private fun getTypeForFile(file : File) : Any? {
        return if (file.isDirectory)
            DocumentsContract.Document.MIME_TYPE_DIR
        else
            getTypeForName(file.name)
    }

    private fun getTypeForName(name : String) : Any? {
        val lastDot = name.lastIndexOf('.')
        if (lastDot >= 0) {
            val extension = name.substring(lastDot + 1)
            val mime = MimeTypeMap.getSingleton().getMimeTypeFromExtension(extension)
            if (mime != null)
                return mime
        }
        return "application/octect-stream"
    }

    override fun queryChildDocuments(parentDocumentId : String?, projection : Array<out String>?, sortOrder : String?) : Cursor {
        var cursor = MatrixCursor(projection ?: DEFAULT_DOCUMENT_PROJECTION)

        val parent = getFile(parentDocumentId!!)
        for (file in parent.listFiles()!!)
            cursor = includeFile(cursor, null, file)

        return cursor
    }

    override fun openDocument(documentId : String?, mode : String?, signal : CancellationSignal?) : ParcelFileDescriptor {
        val file = documentId?.let { getFile(it) }
        val accessMode = ParcelFileDescriptor.parseMode(mode)
        return ParcelFileDescriptor.open(file, accessMode)
    }
}
