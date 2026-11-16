package io.github.antoinecellerier.dmmtools;

import android.app.NativeActivity;
import android.app.PendingIntent;
import android.content.ActivityNotFoundException;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.res.XmlResourceParser;
import android.database.Cursor;
import android.hardware.usb.UsbDevice;
import android.hardware.usb.UsbDeviceConnection;
import android.hardware.usb.UsbManager;
import android.net.Uri;
import android.os.Build;
import android.os.ParcelFileDescriptor;
import android.provider.DocumentsContract;
import android.provider.OpenableColumns;
import android.view.Display;
import android.view.RoundedCorner;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.HashSet;
import java.util.Set;
import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.BlockingQueue;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import org.xmlpull.v1.XmlPullParser;
import org.xmlpull.v1.XmlPullParserException;

/** The activity the native library (crates/dmm-android) draws in. */
public class MainActivity extends NativeActivity {
    private static final int PICK_FILE = 1;
    /** What the picker answered: the document, or this when none was picked. */
    private static final Uri NO_FILE = Uri.EMPTY;
    private final BlockingQueue<Uri> picked = new ArrayBlockingQueue<>(1);

    /** What openUsb answers in place of a descriptor (open_usb in crates/dmm-android). */
    private static final int USB_GONE = -1;
    private static final int USB_REFUSED = -2;
    private static final int USB_NOT_OPENED = -3;
    private static final String USB_PERMISSION =
            "io.github.antoinecellerier.dmmtools.USB_PERMISSION";
    /** How long the permission dialog may stay unanswered. */
    private static final long USB_ANSWER_SECONDS = 60;
    private final BlockingQueue<Boolean> usbAnswered = new ArrayBlockingQueue<>(1);
    /**
     * The devices not to ask the user about, by name, until they are
     * unplugged. One the user did not allow: a session that retries would
     * ask again every time. And one plugged in while the application runs:
     * the system asks about it itself, offering the application for it
     * (the manifest's USB_DEVICE_ATTACHED filter), and a request here would
     * put a second dialog over that one.
     */
    private final Set<String> usbNotAsked = ConcurrentHashMap.newKeySet();
    private BroadcastReceiver usbReceiver;
    /**
     * The devices plugged in while the application runs whose offer by the
     * system is not known to be answered.
     */
    private final Set<String> usbOfferPending = ConcurrentHashMap.newKeySet();
    /**
     * Set when the system's offer of a cable just plugged in was answered:
     * taken, which starts the activity for the cable, or turned down.
     */
    private final AtomicBoolean cableOffered = new AtomicBoolean();
    /** The cables of res/xml/usb_cables.xml as "vendor:product", once read. */
    private Set<String> cableIds;

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
     * The system started the application for a cable (PermissionActivity
     * passes the intent on) while it was running: the native side is told
     * when it next asks. A cable that starts the application needs no
     * telling, as a start looks for one.
     */
    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        if (UsbManager.ACTION_USB_DEVICE_ATTACHED.equals(intent.getAction())) {
            cableOffered.set(true);
        }
    }

    /**
     * The system tells nobody that its offer of a cable was turned down.
     * Its dialog takes the focus, so a cable still not allowed when the
     * focus is back is one whose offer was: the native side is told, and
     * finds out why the cable does not open.
     */
    @Override
    public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus);
        UsbManager manager = (UsbManager) getSystemService(Context.USB_SERVICE);
        if (!hasFocus || manager == null) {
            return;
        }
        for (String name : usbOfferPending) {
            usbOfferPending.remove(name);
            UsbDevice device = manager.getDeviceList().get(name);
            if (device != null && !manager.hasPermission(device)) {
                cableOffered.set(true);
            }
        }
    }

    /**
     * Whether the system's offer of a cable was answered since this was
     * last asked: the cable is the application's to open, or the user
     * turned it down. Called by the native side every frame.
     */
    public boolean takeCableOffered() {
        return cableOffered.getAndSet(false);
    }

    /**
     * The USB devices attached, for the native side to find its cables
     * among: one line each, of the device's name, vendor id, product id and
     * product name, tab separated.
     */
    public String usbCables() {
        listenForUsb();
        UsbManager manager = (UsbManager) getSystemService(Context.USB_SERVICE);
        StringBuilder lines = new StringBuilder();
        if (manager == null) {
            return "";
        }
        for (UsbDevice device : manager.getDeviceList().values()) {
            String product = device.getProductName();
            lines.append(device.getDeviceName())
                    .append('\t').append(device.getVendorId())
                    .append('\t').append(device.getProductId())
                    .append('\t').append(product == null ? "" : product.replaceAll("[\t\n]", " "))
                    .append('\n');
        }
        return lines.toString();
    }

    /**
     * Opens the USB device of this name for the native side, asking the
     * user first where Android wants that, and waiting for the answer.
     * Called on the native side's acquisition thread. Returns a file
     * descriptor the caller owns, or a negative number: the device is gone,
     * was not allowed, or did not open.
     */
    public int openUsb(String name) throws InterruptedException {
        UsbManager manager = (UsbManager) getSystemService(Context.USB_SERVICE);
        UsbDevice device = manager == null ? null : manager.getDeviceList().get(name);
        if (device == null) {
            return USB_GONE;
        }
        if (!manager.hasPermission(device)) {
            if (usbNotAsked.contains(name)) {
                return USB_REFUSED;
            }
            usbAnswered.clear();
            listenForUsb();
            // An intent for this application alone, as a mutable one must
            // be; mutable for the system to add its answer.
            Intent answer = new Intent(USB_PERMISSION).setPackage(getPackageName());
            int flags = Build.VERSION.SDK_INT >= Build.VERSION_CODES.S
                    ? PendingIntent.FLAG_MUTABLE : 0;
            manager.requestPermission(device, PendingIntent.getBroadcast(this, 0, answer, flags));
            usbAnswered.poll(USB_ANSWER_SECONDS, TimeUnit.SECONDS);
            if (!manager.hasPermission(device)) {
                usbNotAsked.add(name);
                return USB_REFUSED;
            }
        }
        UsbDeviceConnection connection = manager.openDevice(device);
        if (connection == null) {
            return USB_NOT_OPENED;
        }
        // A duplicate for the native side to own and close: the connection
        // would close the descriptor under it.
        try (ParcelFileDescriptor duplicate =
                ParcelFileDescriptor.fromFd(connection.getFileDescriptor())) {
            return duplicate.detachFd();
        } catch (IOException e) {
            return USB_NOT_OPENED;
        } finally {
            connection.close();
        }
    }

    /**
     * Registers, once, the receiver of the permission dialog's answer and
     * of devices coming and going. Called before the first listing, so a
     * device heard of here was plugged in after it.
     */
    private synchronized void listenForUsb() {
        if (usbReceiver != null) {
            return;
        }
        usbReceiver = new BroadcastReceiver() {
            @Override
            public void onReceive(Context context, Intent intent) {
                String action = intent.getAction();
                if (UsbManager.ACTION_USB_DEVICE_ATTACHED.equals(action)
                        || UsbManager.ACTION_USB_DEVICE_DETACHED.equals(action)) {
                    UsbDevice device = intent.getParcelableExtra(UsbManager.EXTRA_DEVICE);
                    if (device == null) {
                        return;
                    }
                    if (UsbManager.ACTION_USB_DEVICE_ATTACHED.equals(action)) {
                        usbNotAsked.add(device.getDeviceName());
                        // The system offers the application for its cables
                        // alone: nothing is to be expected of a mouse.
                        if (isCable(device)) {
                            usbOfferPending.add(device.getDeviceName());
                        }
                    } else {
                        usbNotAsked.remove(device.getDeviceName());
                        usbOfferPending.remove(device.getDeviceName());
                    }
                    return;
                }
                // What was answered is asked of the UsbManager.
                usbAnswered.offer(Boolean.TRUE);
            }
        };
        IntentFilter filter = new IntentFilter(USB_PERMISSION);
        filter.addAction(UsbManager.ACTION_USB_DEVICE_ATTACHED);
        filter.addAction(UsbManager.ACTION_USB_DEVICE_DETACHED);
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            registerReceiver(usbReceiver, filter, Context.RECEIVER_NOT_EXPORTED);
        } else {
            registerReceiver(usbReceiver, filter);
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

    /**
     * Whether the device is one of the cables res/xml/usb_cables.xml lists,
     * the ones the system offers the application for.
     */
    private synchronized boolean isCable(UsbDevice device) {
        if (cableIds == null) {
            Set<String> ids = new HashSet<>();
            try (XmlResourceParser xml = getResources().getXml(R.xml.usb_cables)) {
                for (int event = xml.next(); event != XmlPullParser.END_DOCUMENT; event = xml.next()) {
                    if (event == XmlPullParser.START_TAG && "usb-device".equals(xml.getName())) {
                        ids.add(xml.getAttributeValue(null, "vendor-id")
                                + ":" + xml.getAttributeValue(null, "product-id"));
                    }
                }
            } catch (XmlPullParserException | IOException e) {
                // Built into the package: it reads, or none is a cable.
            }
            cableIds = ids;
        }
        return cableIds.contains(device.getVendorId() + ":" + device.getProductId());
    }

    /**
     * Copies a picked document into the cache folder, under its own name,
     * in place of the one copied before: one import is open at a time.
     */
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
        File[] earlier = dir.listFiles();
        if (earlier != null) {
            for (File file : earlier) {
                file.delete();
            }
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
