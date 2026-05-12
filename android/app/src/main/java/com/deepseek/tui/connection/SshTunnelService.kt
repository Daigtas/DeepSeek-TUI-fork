package com.deepseek.tui.connection

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.os.IBinder
import androidx.core.app.NotificationCompat
import com.deepseek.tui.MainActivity
import com.deepseek.tui.R

/**
 * Android foreground service that holds the SSH tunnel alive
 * even when the app is backgrounded. Shows a persistent
 * notification while connected.
 *
 * The actual tunnel management is delegated to SshTunnelManager;
 * this service is a lifecycle wrapper.
 */
class SshTunnelService : Service() {

    companion object {
        const val CHANNEL_ID = "deepseek_ssh_tunnel"
        const val NOTIFICATION_ID = 1001
        const val ACTION_CONNECT = "com.deepseek.tui.action.CONNECT"
        const val ACTION_DISCONNECT = "com.deepseek.tui.action.DISCONNECT"
    }

    private lateinit var tunnelManager: SshTunnelManager

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
        tunnelManager = (application as com.deepseek.tui.DeepSeekApp).appContainer.sshTunnelManager
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_CONNECT -> {
                startForeground(NOTIFICATION_ID, buildNotification("Connecting…"))
                // Connection is triggered by the ViewModel; this service
                // just provides the foreground lifecycle.
            }
            ACTION_DISCONNECT -> {
                tunnelManager.disconnect()
                stopForeground(STOP_FOREGROUND_REMOVE)
                stopSelf()
            }
        }
        return START_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onDestroy() {
        tunnelManager.disconnect()
        super.onDestroy()
    }

    fun updateNotification(text: String) {
        val notification = buildNotification(text)
        val nm = getSystemService(NOTIFICATION_SERVICE) as NotificationManager
        nm.notify(NOTIFICATION_ID, notification)
    }

    private fun buildNotification(text: String): Notification {
        val pendingIntent = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val disconnectIntent = PendingIntent.getService(
            this,
            1,
            Intent(this, SshTunnelService::class.java).apply {
                action = ACTION_DISCONNECT
            },
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle(getString(R.string.ssh_tunnel_notification_title))
            .setContentText(text)
            .setSmallIcon(android.R.drawable.ic_menu_manage)
            .setContentIntent(pendingIntent)
            .addAction(android.R.drawable.ic_menu_close_clear_cancel, "Disconnect", disconnectIntent)
            .setOngoing(true)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .build()
    }

    private fun createNotificationChannel() {
        val channel = NotificationChannel(
            CHANNEL_ID,
            getString(R.string.ssh_tunnel_notification_channel),
            NotificationManager.IMPORTANCE_LOW
        ).apply {
            description = "Persistent notification while SSH tunnel is active"
            setShowBadge(false)
        }
        val nm = getSystemService(NOTIFICATION_SERVICE) as NotificationManager
        nm.createNotificationChannel(channel)
    }
}
