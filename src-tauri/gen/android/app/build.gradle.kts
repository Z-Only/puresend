plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.mmz.puresend"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.mmz.puresend"
        minSdk = 24
        targetSdk = 34
        versionCode = 1
        versionName = "0.1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"

        // 使用 ndk.abiFilters 替代废弃的 splits.abi 配置
        // 支持的 ABI 架构
        ndk {
            abiFilters += listOf("armeabi-v7a", "arm64-v8a", "x86", "x86_64")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
        debug {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    buildFeatures {
        buildConfig = true
    }
}

// 为 Tauri CLI 兼容性添加 assembleUniversalRelease 任务别名
// 使用 afterEvaluate 确保在配置阶段完成后创建任务依赖
afterEvaluate {
    tasks.findByName("assembleRelease")?.let {
        tasks.register("assembleUniversalRelease") {
            group = "build"
            description = "Assemble universal release APK (alias for assembleRelease)"
            dependsOn(it)
        }
    }
    tasks.findByName("assembleDebug")?.let {
        tasks.register("assembleUniversalDebug") {
            group = "build"
            description = "Assemble universal debug APK (alias for assembleDebug)"
            dependsOn(it)
        }
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.12.0")
    implementation("androidx.appcompat:appcompat:1.6.1")
    implementation("com.google.android.material:material:1.11.0")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.5")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.1")
}