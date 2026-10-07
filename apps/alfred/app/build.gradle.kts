plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}
val nativeAbi = providers.gradleProperty("nativeAbi").orElse("arm64-v8a").get()
require(nativeAbi in setOf("arm64-v8a", "x86_64"))

android {
    namespace = "dev.alfred.workspace"
    compileSdk = 36
    buildToolsVersion = "36.0.0"
    ndkVersion = "30.0.16248370"
    defaultConfig {
        applicationId = "dev.alfred.workspace.debug"
        minSdk = 30
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0-a03"
        ndk { abiFilters += nativeAbi }
    }
    signingConfigs.getByName("debug") {
        storeFile = rootProject.file("../../.tools/alfred/debug.keystore")
    }
    buildFeatures { compose = true }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    sourceSets["main"].jniLibs.srcDir(layout.buildDirectory.dir("generated/jniLibs"))
    packaging { jniLibs.useLegacyPackaging = false }
}
kotlin { compilerOptions { jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17) } }

// No APK may silently omit native code. The build driver produces this file first.
val requireNative by tasks.registering {
    doLast {
        check(layout.buildDirectory.file("generated/jniLibs/$nativeAbi/libalfred_native.so").get().asFile.isFile) {
            "Missing $nativeAbi native library. Run scripts/build.ps1 (see BUILDING.md)."
        }
    }
}
tasks.named("preBuild") { dependsOn(requireNative) }
// Public generated signals only. This harness/activity is absent from release.
val smokeAssets by tasks.registering(Copy::class) {
    from(rootProject.file("../../tests/fixtures")) {
        include("noise16.wav", "noise16.flac", "alac/8000-16-1-tail.m4a")
    }
    into(layout.buildDirectory.dir("generated/smokeAssets"))
}
// Pass the producer, not a bare directory, so lint as well as packaging inherits
// the generated-assets dependency.
android.sourceSets["debug"].assets.srcDir(smokeAssets)
dependencies {
    implementation(project(":shared"))
    implementation(project(":feature-forensics"))
    implementation(project(":feature-spectrogram"))
    implementation(project(":feature-compare"))
    implementation("androidx.activity:activity-compose:1.10.1")
    implementation("androidx.compose.material3:material3:1.3.2")
}
