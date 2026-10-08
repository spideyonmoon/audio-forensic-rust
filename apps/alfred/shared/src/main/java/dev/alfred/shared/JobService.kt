package dev.alfred.shared

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.os.PowerManager

/** Same process, explicit visible-user start only. No sticky restart or boot receiver. */
class JobService : Service() {
    private val handler = Handler(Looper.getMainLooper())
    private lateinit var jobs: SharedJobs
    private var wake: PowerManager.WakeLock? = null
    private var promoted = false
    private var notifiedAttempt: String? = null
    private val tick = object : Runnable {
        override fun run() {
            if (!jobs.busy()) { stopForeground(STOP_FOREGROUND_REMOVE); stopSelf(); return }
            if (jobs.releaseUnconfirmed()) { timeout(); return }
            if (jobs.expired()) { timeout(); return }
            val attempt = jobs.snapshot().firstOrNull { !it.terminal && it.state != "queued" }?.attemptId
                ?: jobs.snapshot().firstOrNull { !it.terminal }?.attemptId
            if (attempt != notifiedAttempt) {
                (getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager).notify(5, notification(attempt))
                notifiedAttempt = attempt
                wake?.let { if (it.isHeld) it.release(); it.acquire(30 * 60 * 1000L) }
            }
            // One latest event, never a frame-sized queue or fabricated percentage.
            handler.postDelayed(this, 250)
        }
    }
    override fun onCreate() { super.onCreate(); jobs = SharedJobs.get(this) }
    override fun onBind(intent: Intent?): IBinder? = null
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == "cancel") {
            intent.getStringExtra("attempt")?.let { jobs.cancel(it) }
            return START_NOT_STICKY
        }
        if (!promoted) {
            try {
                val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
                manager.createNotificationChannel(NotificationChannel("audio-jobs", "Audio operations", NotificationManager.IMPORTANCE_LOW))
                val attempt = jobs.snapshot().firstOrNull { !it.terminal }?.attemptId
                val notification = notification(attempt)
                notifiedAttempt = attempt
                when {
                    Build.VERSION.SDK_INT >= 35 -> startForeground(5, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PROCESSING)
                    Build.VERSION.SDK_INT >= 34 -> startForeground(5, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE)
                    else -> startForeground(5, notification)
                }
                wake = (getSystemService(Context.POWER_SERVICE) as PowerManager).newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "Alfred:audio-jobs")
                    .apply { acquire(30 * 60 * 1000L) }
                promoted = true
                handler.post(tick)
            } catch (_: RuntimeException) {
                jobs.interrupt("service_start_rejected"); stopSelf(); return START_NOT_STICKY
            }
        }
        jobs.promoted()
        return START_NOT_STICKY
    }
    private fun notification(attempt: String?): Notification {
        val cancel = PendingIntent.getService(this, 1, Intent(this, JobService::class.java).setAction("cancel").putExtra("attempt", attempt),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
        return Notification.Builder(this, "audio-jobs").setSmallIcon(android.R.drawable.stat_notify_sync)
            .setContentTitle("Alfred audio operation").setContentText("Working on this device; cancellation may wait for the reader")
            .setOngoing(true).addAction(Notification.Action.Builder(null, "Cancel", cancel).build()).build()
    }
    private fun timeout() {
        jobs.interrupt("interrupted")
        stopForeground(STOP_FOREGROUND_REMOVE)
        wake?.let { if (it.isHeld) it.release() }; wake = null
        stopSelf()
    }
    override fun onTimeout(startId: Int, fgsType: Int) { timeout() }
    override fun onDestroy() {
        handler.removeCallbacks(tick)
        if (jobs.running()) jobs.interrupt()
        wake?.let { if (it.isHeld) it.release() }; wake = null
        jobs.serviceDetached()
        super.onDestroy()
    }
}
