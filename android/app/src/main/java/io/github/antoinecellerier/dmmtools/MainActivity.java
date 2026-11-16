package io.github.antoinecellerier.dmmtools;

import android.app.NativeActivity;
import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.os.Build;
import android.provider.DocumentsContract;
import android.provider.OpenableColumns;
import android.view.Display;
import android.view.RoundedCorner;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.BlockingQueue;

/** The activity the native library (crates/dmm-android) draws in. */
public class MainActivity extends NativeActivity {
    private static final int PICK_FILE = 1;
    /** What the picker answered: the document, or this when none was picked. */
    private static final Uri NO_FILE = Uri.EMPTY;
    private final BlockingQueue<Uri> picked = new ArrayBlockingQueue<>(1);

    /**
     * Shows the system's document picker and waits for the answer. Called by
     * the native side, on a thread of its own, for Import. Returns the path
     * of a copy of the document in the cache folder, as a document has no
     * path the native side could open, or null when none was picked.
     */
    public String pickFile() throws InterruptedException {
        picked.clear();
        runOnUiThread(() -> {
            Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
            intent.addCategory(Intent.CATEGORY_OPENABLE);
            intent.setType("*/*");
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                // A hint the picker may ignore: start where exports are
                // saved (android_save_folder in crates/dmm-gui/src/lib.rs).
                intent.putExtra(
                        DocumentsContract.EXTRA_INITIAL_URI,
                        DocumentsContract.buildDocumentUri(
                                "com.android.externalstorage.documents",
                                "primary:Documents/dmm-tools"));
            }
            try {
                startActivityForResult(intent, PICK_FILE);
            } catch (ActivityNotFoundException e) {
                picked.offer(NO_FILE);
            }
        });
        Uri document = picked.take();
        if (document == NO_FILE) {
            return null;
        }
        try {
            return copyToCache(document).getPath();
        } catch (IOException e) {
            return null;
        }
    }

    /**
     * The radius of the screen's rounded corners in pixels, 0 for square
     * ones, for the native side to keep its drawing clear of them: full
     * screen, the window reaches into the corners.
     */
    public int screenCornerRadius() {
        int radius = 0;
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            Display display = getDisplay();
            for (int position = 0; display != null && position < 4; position++) {
                RoundedCorner corner = display.getRoundedCorner(position);
                if (corner != null) {
                    radius = Math.max(radius, corner.getRadius());
                }
            }
        }
        if (radius == 0) {
            // Before Android 12, and where a device's corners are only in
            // its system resources.
            int id = getResources().getIdentifier("rounded_corner_radius", "dimen", "android");
            if (id != 0) {
                radius = getResources().getDimensionPixelSize(id);
            }
        }
        return radius;
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        if (requestCode != PICK_FILE) {
            super.onActivityResult(requestCode, resultCode, data);
            return;
        }
        Uri document = resultCode == RESULT_OK && data != null ? data.getData() : null;
        picked.offer(document != null ? document : NO_FILE);
    }

    /** Copies a picked document into the cache folder, under its own name. */
    private File copyToCache(Uri document) throws IOException {
        String name = "import";
        try (Cursor cursor = getContentResolver().query(
                document, new String[] {OpenableColumns.DISPLAY_NAME}, null, null, null)) {
            if (cursor != null && cursor.moveToFirst() && !cursor.isNull(0)) {
                // The last path segment only: the name is the provider's.
                name = new File(cursor.getString(0)).getName();
            }
        }
        File dir = new File(getCacheDir(), "import");
        if (!dir.isDirectory() && !dir.mkdirs()) {
            throw new IOException("cannot create " + dir);
        }
        File copy = new File(dir, name);
        try (InputStream in = getContentResolver().openInputStream(document);
                OutputStream out = new FileOutputStream(copy)) {
            if (in == null) {
                throw new IOException("cannot open " + document);
            }
            byte[] buffer = new byte[65536];
            for (int n; (n = in.read(buffer)) > 0; ) {
                out.write(buffer, 0, n);
            }
        }
        return copy;
    }

    /**
     * Ends the process with the activity. The native event loop does not
     * end when the activity is destroyed, the destroy callback waits for it,
     * and it can be built only once per process: left alive, closing would
     * hang and the next launch could not start.
     */
    @Override
    protected void onDestroy() {
        if (isFinishing()) {
            System.exit(0);
        }
        super.onDestroy();
    }
}
