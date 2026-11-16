package io.github.antoinecellerier.dmmtools;

import android.Manifest;
import android.app.Activity;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.os.Build;
import android.os.Bundle;

/**
 * Asks for the Bluetooth permissions, then starts the application, granted
 * or not. A native activity is handed no permission result, so asking here
 * means the native side never runs before the dialog is answered; refused,
 * it reports what Bluetooth then tells it.
 */
public class PermissionActivity extends Activity {
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        String[] wanted = Build.VERSION.SDK_INT >= Build.VERSION_CODES.S
                ? new String[] {Manifest.permission.BLUETOOTH_SCAN, Manifest.permission.BLUETOOTH_CONNECT}
                : new String[] {Manifest.permission.ACCESS_FINE_LOCATION};
        for (String permission : wanted) {
            if (checkSelfPermission(permission) != PackageManager.PERMISSION_GRANTED) {
                requestPermissions(wanted, 0);
                return;
            }
        }
        startApplication();
    }

    @Override
    public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] grantResults) {
        startApplication();
    }

    private void startApplication() {
        // The action says what started this one: the launcher, or a USB
        // cable the system offered the application for.
        startActivity(new Intent(this, MainActivity.class).setAction(getIntent().getAction()));
        finish();
    }
}
