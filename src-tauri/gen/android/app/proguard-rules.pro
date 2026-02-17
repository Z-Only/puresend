# Add project specific ProGuard rules here.
# By default, the flags in this file are appended to flags specified
# in the Android SDK.
# For more details, see
#   https://developer.android.com/build/shrink-code

# Keep Tauri related classes
-keep class com.tauri.** { *; }
-keep class com.mmz.puresend.** { *; }

# Keep native methods
-keepclasseswithmembernames class * {
    native &lt;methods&gt;;
}
