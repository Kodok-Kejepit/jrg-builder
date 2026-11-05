package jrg.ide.latar;

// JRG IDE Terminal — layanan latar seperti Termux: notifikasi "N sesi berjalan" dengan Keluar dan Wake lock,
// supaya Android tidak mematikan aplikasinya (dan sesi terminalnya) saat ditinggal. Disisipkan bangun_apk.sh
// sebagai classes2.dex; halaman memanggilnya lewat `JRGLatar` (ide/README.md, "Jalan di latar").

import android.app.Activity;
import android.app.Application;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.content.ContentProvider;
import android.content.ContentValues;
import android.content.Context;
import android.content.Intent;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.database.Cursor;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.IBinder;
import android.os.PowerManager;
import android.view.View;
import android.view.ViewGroup;
import android.webkit.JavascriptInterface;
import android.webkit.WebView;

public class Latar {
    static final String SALURAN = "jrg-latar";
    static final String KELUAR = "jrg.ide.latar.KELUAR";
    static final String KUNCI = "jrg.ide.latar.KUNCI";
    static final String TEKS = "teks";
    static Activity aktif;
    static String teks = "";
    static boolean minta, terjeda;

    static int port(Context c) {
        try {
            ApplicationInfo ai = c.getPackageManager().getApplicationInfo(c.getPackageName(), PackageManager.GET_META_DATA);
            return ai.metaData == null ? 8080 : Integer.parseInt(String.valueOf(ai.metaData.get("jrg.port")));
        } catch (Exception e) { return 8080; }
    }

    static WebView cariWeb(View v) {
        if (v instanceof WebView) return (WebView) v;
        if (v instanceof ViewGroup) {
            ViewGroup g = (ViewGroup) v;
            for (int i = 0; i < g.getChildCount(); i++) { WebView w = cariWeb(g.getChildAt(i)); if (w != null) return w; }
        }
        return null;
    }

    static void kabar(final Activity a, final String js) {
        a.runOnUiThread(new Runnable() { public void run() {
            WebView w = cariWeb(a.getWindow().getDecorView());
            if (w != null) w.evaluateJavascript(js, null);
        } });
    }

    static boolean diberi(Context c, String izin) { return c.checkSelfPermission(izin) == PackageManager.PERMISSION_GRANTED; }

    // Dimuat Android saat proses aplikasi mulai, sebelum aktivitasnya: tempat memasang pintu `JRGLatar`.
    public static class Penyedia extends ContentProvider {
        @Override
        public boolean onCreate() {
            Application app = (Application) getContext().getApplicationContext();
            app.registerActivityLifecycleCallbacks(new Application.ActivityLifecycleCallbacks() {
                public void onActivityCreated(Activity a, Bundle b) { aktif = a; }
                public void onActivityStarted(Activity a) {
                    aktif = a;
                    WebView w = cariWeb(a.getWindow().getDecorView());
                    if (w != null && w.getTag(0x7f0f1a7a) == null) {
                        w.setTag(0x7f0f1a7a, Boolean.TRUE);
                        w.addJavascriptInterface(new Pintu(a), "JRGLatar");
                    }
                }
                // Dialog izin menutupi aktivitas; jawabannya terbaca saat aktivitas kembali di depan.
                public void onActivityResumed(Activity a) { aktif = a; if (minta && terjeda) jawabIzin(a); }
                public void onActivityPaused(Activity a) { if (minta) terjeda = true; }
                public void onActivityStopped(Activity a) {}
                public void onActivitySaveInstanceState(Activity a, Bundle b) {}
                public void onActivityDestroyed(Activity a) { if (aktif == a) aktif = null; }
            });
            return true;
        }
        public Cursor query(Uri u, String[] p, String s, String[] a, String o) { return null; }
        public String getType(Uri u) { return null; }
        public Uri insert(Uri u, ContentValues v) { return null; }
        public int delete(Uri u, String s, String[] a) { return 0; }
        public int update(Uri u, ContentValues v, String s, String[] a) { return 0; }
    }

    static void jawabIzin(Activity a) {
        minta = false;
        terjeda = false;
        boolean ya = diberi(a, "android.permission.WRITE_EXTERNAL_STORAGE");
        kabar(a, "window.dispatchEvent(new CustomEvent('jrg-izin',{detail:{nama:'penyimpanan',diberi:" + ya + "}}))");
    }

    // `window.JRGLatar` di halaman.
    public static class Pintu {
        final Activity a;
        Pintu(Activity a) { this.a = a; }

        // Teks kosong menghentikan layanan (tidak ada sesi yang perlu dijaga).
        @JavascriptInterface
        public void latar(String t) {
            Intent i = new Intent(a, Layanan.class);
            if (t == null || t.isEmpty()) { a.stopService(i); return; }
            i.putExtra(TEKS, t);
            if (Build.VERSION.SDK_INT >= 26) a.startForegroundService(i); else a.startService(i);
        }

        @JavascriptInterface
        public String izin(String nama) {
            if (!"penyimpanan".equals(nama)) return "tidak_dikenal";
            return diberi(a, "android.permission.WRITE_EXTERNAL_STORAGE") ? "diberi" : "belum";
        }

        // Baca dan tulis memori HP dalam satu dialog; jawabannya peristiwa `jrg-izin` seperti JRGAndroid.
        @JavascriptInterface
        public String mintaIzin(String nama) {
            String kini = izin(nama);
            if (!kini.equals("belum")) return kini;
            minta = true;
            terjeda = false;
            a.runOnUiThread(new Runnable() { public void run() {
                a.requestPermissions(new String[] { "android.permission.READ_EXTERNAL_STORAGE", "android.permission.WRITE_EXTERNAL_STORAGE" }, 7707);
                // Sudah ditolak permanen: Android menjawab tanpa dialog, jadi aktivitasnya tidak pernah jeda.
                a.getWindow().getDecorView().postDelayed(new Runnable() { public void run() { if (minta && !terjeda) jawabIzin(a); } }, 1500);
            } });
            return "ditanyakan";
        }

        @JavascriptInterface
        public void pengaturan() {
            Intent i = new Intent(android.provider.Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.parse("package:" + a.getPackageName()));
            i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            a.startActivity(i);
        }
    }

    public static class Layanan extends Service {
        PowerManager.WakeLock kunci;

        @Override
        public IBinder onBind(Intent i) { return null; }

        @Override
        public int onStartCommand(Intent i, int flags, int id) {
            String aksi = i == null ? null : i.getAction();
            if (KELUAR.equals(aksi)) { keluar(); return START_NOT_STICKY; }
            if (KUNCI.equals(aksi)) {
                if (kunci != null) { kunci.release(); kunci = null; }
                else {
                    kunci = ((PowerManager) getSystemService(POWER_SERVICE)).newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "jrg-ide:latar");
                    kunci.acquire();
                }
            } else if (i != null && i.getStringExtra(TEKS) != null) {
                teks = i.getStringExtra(TEKS);
            }
            startForeground(1, notifikasi());
            return START_NOT_STICKY;
        }

        Notification notifikasi() {
            NotificationManager nm = (NotificationManager) getSystemService(NOTIFICATION_SERVICE);
            Notification.Builder b;
            if (Build.VERSION.SDK_INT >= 26) {
                nm.createNotificationChannel(new NotificationChannel(SALURAN, "Sesi berjalan", NotificationManager.IMPORTANCE_LOW));
                b = new Notification.Builder(this, SALURAN);
            } else {
                b = new Notification.Builder(this);
            }
            int imut = Build.VERSION.SDK_INT >= 23 ? PendingIntent.FLAG_IMMUTABLE : 0;
            Intent buka = getPackageManager().getLaunchIntentForPackage(getPackageName());
            PendingIntent pb = PendingIntent.getActivity(this, 0, buka, imut);
            PendingIntent pk = PendingIntent.getService(this, 1, new Intent(this, Layanan.class).setAction(KELUAR), imut);
            PendingIntent pw = PendingIntent.getService(this, 2, new Intent(this, Layanan.class).setAction(KUNCI), imut);
            String isi = teks.isEmpty() ? "Berjalan" : teks;
            if (kunci != null) isi += " · wake lock";
            int ikon = getApplicationInfo().icon;
            return b.setSmallIcon(ikon != 0 ? ikon : android.R.drawable.stat_notify_more)
                .setContentTitle(getApplicationInfo().loadLabel(getPackageManager()))
                .setContentText(isi)
                .setOngoing(true)
                .setContentIntent(pb)
                .addAction(0, "Keluar", pk)
                .addAction(0, kunci == null ? "Wake lock" : "Lepas wake lock", pw)
                .build();
        }

        // Keluar sungguhan: pelayan (dan semua sesinya) dihentikan, lalu aplikasinya.
        void keluar() {
            new Thread(new Runnable() { public void run() {
                try {
                    java.net.HttpURLConnection h = (java.net.HttpURLConnection) new java.net.URL("http://127.0.0.1:" + port(Layanan.this) + "/api/keluar").openConnection();
                    h.setRequestMethod("POST");
                    h.setConnectTimeout(2000);
                    h.setReadTimeout(2000);
                    h.getResponseCode();
                } catch (Exception e) { /* pelayannya sudah mati */ }
                if (aktif != null) aktif.runOnUiThread(new Runnable() { public void run() { if (aktif != null) aktif.finishAndRemoveTask(); } });
                try { Thread.sleep(600); } catch (InterruptedException e) { /* lanjut */ }
                android.os.Process.killProcess(android.os.Process.myPid());
            } }).start();
            if (kunci != null) { kunci.release(); kunci = null; }
            stopForeground(true);
            stopSelf();
        }

        @Override
        public void onDestroy() {
            if (kunci != null) { kunci.release(); kunci = null; }
            super.onDestroy();
        }
    }
}
