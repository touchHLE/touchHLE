/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 *
 * Parts of this file are derived from SDL 2's Android project template, which
 * has a different license. Please see vendor/SDL/LICENSE.txt for details.
 */
package org.touchhle.android;

import android.content.Context;
import android.content.Intent;
import android.net.Uri;
import android.util.Log;

import androidx.core.content.FileProvider;

import org.libsdl.app.SDLActivity;

import java.io.File;

/**
 * A wrapper class over SDLActivity
 */

public class MainActivity extends SDLActivity {
    /**
     * File name the native in-app updater stages the downloaded APK under, in
     * the app's external files directory. Must match APK_FILE_NAME in
     * src/update.rs.
     */
    private static final String UPDATE_APK_NAME = "HyperHLE_update.apk";

    @Override
    protected String[] getLibraries() {
        return new String[]{
            "SDL2",
            "touchHLE"
        };
    }

    /**
     * Hand the downloaded update APK to the system package installer. Called
     * from native code (src/update.rs) via JNI after the APK has been staged in
     * the app's external files directory.
     *
     * An Android app cannot replace its own installed binary by writing files,
     * so the system installer (with the user's confirmation) does it instead.
     */
    public static void installApk() {
        Context context = getContext();
        if (context == null) {
            Log.e("touchHLE", "installApk: no application context");
            return;
        }
        File apk = new File(context.getExternalFilesDir(null), UPDATE_APK_NAME);
        if (!apk.isFile()) {
            Log.e("touchHLE", "installApk: APK not found at " + apk.getAbsolutePath());
            return;
        }
        try {
            Uri uri = FileProvider.getUriForFile(
                context, context.getPackageName() + ".fileprovider", apk);
            Intent intent = new Intent(Intent.ACTION_VIEW);
            intent.setDataAndType(uri, "application/vnd.android.package-archive");
            intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            context.startActivity(intent);
        } catch (Exception e) {
            Log.e("touchHLE", "installApk failed", e);
        }
    }
}
