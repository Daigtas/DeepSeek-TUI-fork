# ── DeepSeek TUI Android — ProGuard / R8 Rules ──────────────────────────────
# Release builds use isMinifyEnabled=true and isShrinkResources=true.
# These rules keep all classes needed at runtime by SSHJ, BouncyCastle,
# OkHttp, Gson, Room, Compose, and the app's own code.
# ────────────────────────────────────────────────────────────────────────────

# ── App package — keep everything ──────────────────────────────────────────
-keep class com.deepseek.tui.** { *; }
-keepclassmembers class com.deepseek.tui.** { *; }

# ── Kotlin ─────────────────────────────────────────────────────────────────
-keepattributes *Annotation*
-keepattributes SourceFile,LineNumberTable
-keep class kotlin.Metadata { *; }
-keep class kotlin.coroutines.** { *; }
-keepclassmembers class kotlinx.coroutines.** {
    volatile <fields>;
}
-dontwarn kotlinx.coroutines.debug.**

# ── SSHJ (SSH client) ──────────────────────────────────────────────────────
-keep class net.schmizz.sshj.** { *; }
-keep class net.schmizz.sshj.transport.** { *; }
-keep class net.schmizz.sshj.connection.** { *; }
-keep class net.schmizz.sshj.userauth.** { *; }
-keep class net.schmizz.sshj.signature.** { *; }
-keep class net.schmizz.sshj.common.** { *; }
-keepclassmembers class net.schmizz.sshj.** {
    <init>(...);
    <fields>;
}
-dontwarn net.schmizz.sshj.**

# ── BouncyCastle (security provider for SSHJ) ──────────────────────────────
-keep class org.bouncycastle.** { *; }
-keep class org.bouncycastle.jcajce.provider.** { *; }
-keep class org.bouncycastle.jce.provider.** { *; }
-keep class org.bouncycastle.crypto.** { *; }
-keep class org.bouncycastle.asn1.** { *; }
-keep class org.bouncycastle.math.** { *; }
-keep class org.bouncycastle.util.** { *; }
-keepclassmembers class org.bouncycastle.** {
    <init>(...);
    <fields>;
}
# Keep JCA/JCE provider registration
-keepclassmembers class * implements java.security.Provider {
    <init>(...);
}
-dontwarn org.bouncycastle.**
-dontwarn org.bouncycastle.jsse.**
# Android does not include sun.* classes (referenced by ed25519 SSH key provider)
-dontwarn sun.security.x509.X509Key

# ── OkHttp + Okio ──────────────────────────────────────────────────────────
-keep class okhttp3.** { *; }
-keep class okio.** { *; }
-keepclassmembers class okhttp3.** {
    <init>(...);
}
-dontwarn okhttp3.**
-dontwarn okio.**
# WebSocket frame classes
-keep class okhttp3.internal.ws.** { *; }

# ── Gson ───────────────────────────────────────────────────────────────────
-keep class com.google.gson.** { *; }
-keepclassmembers class com.google.gson.** {
    <init>(...);
}
# Keep Gson TypeToken (used for JSON deserialization)
-keep class com.google.gson.reflect.TypeToken { *; }
-keep class * extends com.google.gson.reflect.TypeToken
# Keep data classes used with Gson
-keepclassmembers class com.deepseek.tui.data.** {
    <fields>;
}
-keepclassmembers class com.deepseek.tui.connection.** {
    <fields>;
}

# ── Room ───────────────────────────────────────────────────────────────────
-keep class androidx.room.** { *; }
-keep class * extends androidx.room.RoomDatabase
-keep @androidx.room.Entity class *
-keep @androidx.room.Dao class *
-keepclassmembers @androidx.room.Entity class * {
    <fields>;
}
-dontwarn androidx.room.**

# ── Compose ────────────────────────────────────────────────────────────────
-keep class androidx.compose.** { *; }
-keepclassmembers class androidx.compose.** {
    <init>(...);
}
-dontwarn androidx.compose.**

# ── AndroidX ───────────────────────────────────────────────────────────────
-keep class androidx.lifecycle.** { *; }
-keep class androidx.activity.** { *; }
-keep class androidx.security.** { *; }
-keep class androidx.navigation.** { *; }
-dontwarn androidx.lifecycle.**
-dontwarn androidx.security.**

# ── Markwon (markdown) ─────────────────────────────────────────────────────
-keep class io.noties.markwon.** { *; }
-dontwarn io.noties.markwon.**

# ── Coroutines ─────────────────────────────────────────────────────────────
-keepnames class kotlinx.coroutines.internal.MainDispatcherFactory {}
-keepnames class kotlinx.coroutines.CoroutineExceptionHandler {}

# ── BuildConfig ────────────────────────────────────────────────────────────
-keep class com.deepseek.tui.BuildConfig { *; }

# ── Misc ───────────────────────────────────────────────────────────────────
# Keep enum values
-keepclassmembers enum * {
    public static **[] values();
    public static ** valueOf(java.lang.String);
}

# Keep serializable classes
-keepclassmembers class * implements java.io.Serializable {
    static final long serialVersionUID;
    private static final java.io.ObjectStreamField[] serialPersistentFields;
    !static !transient <fields>;
    private void writeObject(java.io.ObjectOutputStream);
    private void readObject(java.io.ObjectInputStream);
    java.lang.Object writeReplace();
    java.lang.Object readResolve();
}

# Strip debug logging in release
-assumenosideeffects class android.util.Log {
    public static int v(...);
    public static int d(...);
}

# ServiceLoader configuration (BouncyCastle uses SPI)
-keepnames class kotlinx.coroutines.internal.MainDispatcherFactory {}
-keepnames class org.bouncycastle.jcajce.provider.** {}
