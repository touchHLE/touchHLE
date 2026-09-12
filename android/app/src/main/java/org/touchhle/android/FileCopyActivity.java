package org.touchhle.android;

import android.app.Activity;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.os.Bundle;
import android.provider.OpenableColumns;
import android.view.View;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ProgressBar;
import android.widget.TextView;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;

public class FileCopyActivity extends Activity {

    private ProgressBar progressBar;
    private TextView statusText;
    private LinearLayout actionsLayout;
    private Button openButton;
    private Button closeButton;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_file_copy);

        setFinishOnTouchOutside(false);

        progressBar = findViewById(R.id.progressBar);
        statusText = findViewById(R.id.statusText);
        actionsLayout = findViewById(R.id.actionsLayout);
        openButton = findViewById(R.id.openButton);
        closeButton = findViewById(R.id.closeButton);

        Uri fileUri = extractIncomingUri(getIntent());
        if (fileUri == null) {
            finish();
            return;
        }

        closeButton.setOnClickListener(v -> finish());

        openButton.setOnClickListener(v -> {
            Intent launchIntent = getPackageManager().getLaunchIntentForPackage(getPackageName());
            if (launchIntent != null) {
                launchIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                startActivity(launchIntent);
            }
            finish();
        });

        copyFileAsync(fileUri);
    }

    private Uri extractIncomingUri(Intent intent) {
        Uri uri = intent.getData();
        if (uri == null && intent.getClipData() != null && intent.getClipData().getItemCount() > 0) {
            uri = intent.getClipData().getItemAt(0).getUri();
        }
        return uri;
    }

    private void copyFileAsync(Uri uri) {
        new Thread(() -> {
            try {
                copyFileToAppFolder(uri, percent -> runOnUiThread(() -> updateProgress(percent)));
                runOnUiThread(this::onCopyFinished);
            } catch (Exception e) {
                runOnUiThread(() -> showError(e));
            }
        }).start();
    }

    private void copyFileToAppFolder(Uri sourceUri, ProgressCallback callback) throws IOException {
        File appsDir = new File(getExternalFilesDir(null), "touchHLE_apps");
        if (!appsDir.exists() && !appsDir.mkdirs()) {
            throw new IOException("Failed to create app directory");
        }

        String fileName = getFileName(sourceUri);
        if (fileName == null) {
            fileName = "imported_file";
        }

        File targetFile = new File(appsDir, fileName);

        try (InputStream in = getContentResolver().openInputStream(sourceUri);
             OutputStream out = new FileOutputStream(targetFile)) {

            if (in == null) throw new IOException("Input stream is null");

            long total = getFileSize(sourceUri);
            long copied = 0;
            byte[] buffer = new byte[8192];
            int read;

            while ((read = in.read(buffer)) != -1) {
                out.write(buffer, 0, read);
                copied += read;
                if (total > 0) {
                    int percent = (int) ((copied * 100) / total);
                    callback.onProgress(percent);
                }
            }
        }
    }

    private void updateProgress(int percent) {
        progressBar.setIndeterminate(false);
        progressBar.setMax(100);
        progressBar.setProgress(percent);
    }

    private void onCopyFinished() {
        progressBar.setVisibility(View.GONE);
        statusText.setText("File imported successfully");
        actionsLayout.setVisibility(View.VISIBLE);
    }

    private void showError(Exception e) {
        progressBar.setVisibility(View.GONE);
        statusText.setText("Failed to import file");
        actionsLayout.setVisibility(View.VISIBLE);
    }

    private String getFileName(Uri uri) {
        if ("content".equals(uri.getScheme())) {
            try (Cursor cursor = getContentResolver().query(uri, null, null, null, null)) {
                if (cursor != null && cursor.moveToFirst()) {
                    int idx = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME);
                    if (idx != -1) {
                        return cursor.getString(idx);
                    }
                }
            }
        }
        return uri.getLastPathSegment();
    }

    private long getFileSize(Uri uri) {
        try (Cursor cursor = getContentResolver().query(uri, null, null, null, null)) {
            if (cursor != null && cursor.moveToFirst()) {
                int idx = cursor.getColumnIndex(OpenableColumns.SIZE);
                if (idx != -1) {
                    return cursor.getLong(idx);
                }
            }
        }
        return -1;
    }

    interface ProgressCallback {
        void onProgress(int percent);
    }
}