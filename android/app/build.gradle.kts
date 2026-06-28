import java.util.Properties
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

val releaseKeystorePath = providers.environmentVariable("AI_TERMINAL_ANDROID_KEYSTORE").orNull
val releaseKeystorePassword = providers.environmentVariable("AI_TERMINAL_ANDROID_KEYSTORE_PASSWORD").orNull
val releaseKeyAlias = providers.environmentVariable("AI_TERMINAL_ANDROID_KEY_ALIAS").orNull
val releaseKeyPassword = providers.environmentVariable("AI_TERMINAL_ANDROID_KEY_PASSWORD").orNull
val releaseSigningConfigured = listOf(
    releaseKeystorePath,
    releaseKeystorePassword,
    releaseKeyAlias,
    releaseKeyPassword,
).all { !it.isNullOrBlank() }
val projectVersion = rootProject.file("../VERSION").readText().trim()

fun androidVersionCode(version: String): Int {
    val match = Regex("""^(\d+)\.(\d+)\.(\d+)(?:[-+].*)?$""").matchEntire(version)
        ?: error("VERSION must be semver-like MAJOR.MINOR.PATCH, got: $version")
    val (major, minor, patch) = match.destructured
    return major.toInt() * 10_000 + minor.toInt() * 100 + patch.toInt()
}

val androidAppVersionCode = androidVersionCode(projectVersion)

android {
    namespace = "dev.aiterminal.android"
    compileSdk = 35

    defaultConfig {
        applicationId = "dev.aiterminal.android"
        minSdk = 26
        targetSdk = 35
        versionCode = androidAppVersionCode
        versionName = projectVersion
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildFeatures {
        compose = true
    }

    signingConfigs {
        if (releaseSigningConfigured) {
            create("release") {
                storeFile = file(releaseKeystorePath!!)
                storePassword = releaseKeystorePassword
                keyAlias = releaseKeyAlias
                keyPassword = releaseKeyPassword
            }
        }
    }

    buildTypes {
        release {
            if (releaseSigningConfigured) {
                signingConfig = signingConfigs.getByName("release")
            }
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDir("src/main/jniLibs")
        }
    }

}

kotlin {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_17)
    }
}

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2024.10.01")
    implementation(composeBom)
    androidTestImplementation(composeBom)

    implementation("androidx.activity:activity-compose:1.9.3")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.8.7")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.7")
    debugImplementation("androidx.compose.ui:ui-tooling")

    testImplementation("junit:junit:4.13.2")
    testImplementation("org.json:json:20250517")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test:runner:1.6.2")
}

tasks.register("verifyNativeLibraries") {
    group = "verification"
    description = "Verify that libai_terminal.so exists for every Android ABI packaged by the spike."

    val expectedAbis = listOf("arm64-v8a", "armeabi-v7a", "x86", "x86_64")
    val jniRoot = layout.projectDirectory.dir("src/main/jniLibs")

    doLast {
        val missing = expectedAbis.filter { abi ->
            !jniRoot.file("$abi/libai_terminal.so").asFile.isFile
        }
        check(missing.isEmpty()) {
            "Missing libai_terminal.so for ABI(s): ${missing.joinToString(", ")}"
        }
    }
}

tasks.register("verifyFdroidReleaseInputs") {
    group = "verification"
    description = "Verify Android release metadata and versioning before F-Droid/direct APK packaging."

    val requiredMetadataFiles = listOf(
        layout.projectDirectory.file("../fastlane/metadata/android/en-US/title.txt"),
        layout.projectDirectory.file("../fastlane/metadata/android/en-US/short_description.txt"),
        layout.projectDirectory.file("../fastlane/metadata/android/en-US/full_description.txt"),
        layout.projectDirectory.file("../fastlane/metadata/android/en-US/changelogs/$androidAppVersionCode.txt"),
    )
    val requiredLicenseFiles = listOf(
        rootProject.file("../LICENSE-MIT"),
        rootProject.file("../LICENSE-APACHE"),
    )
    val fdroidVersionFile = rootProject.file("fdroid-version.properties")
    val fdroidDataMetadataFile = rootProject.file("fdroiddata/metadata/dev.aiterminal.android.yml")
    val fdroidMetadataSmokeScript = rootProject.file("smoke-fdroid-metadata.ps1")
    val fdroidReleaseActivationSmokeScript = rootProject.file("smoke-fdroid-release-activation.ps1")
    val githubSigningSecretsSmokeScript = rootProject.file("smoke-github-signing-secrets.ps1")
    val screenshotDir = layout.projectDirectory.dir("../fastlane/metadata/android/en-US/images/phoneScreenshots")
    val minimumScreenshotCount = 2

    doLast {
        check(projectVersion.isNotBlank()) { "VERSION is empty" }
        check(androidAppVersionCode > 0) { "Android versionCode must be positive" }
        requiredMetadataFiles.forEach { file ->
            check(file.asFile.isFile) { "Missing Android release metadata file: ${file.asFile}" }
            check(file.asFile.readText().trim().isNotEmpty()) {
                "Android release metadata file is empty: ${file.asFile}"
            }
        }
        requiredLicenseFiles.forEach { file ->
            check(file.isFile) { "Missing repository license file: $file" }
            check(file.readText().trim().isNotEmpty()) { "Repository license file is empty: $file" }
        }
        check(fdroidVersionFile.isFile) { "Missing F-Droid version metadata file: $fdroidVersionFile" }
        val fdroidVersionProperties = Properties().apply {
            fdroidVersionFile.inputStream().use { stream -> load(stream) }
        }
        check(fdroidVersionProperties.getProperty("versionName") == projectVersion) {
            "F-Droid versionName must match VERSION: ${fdroidVersionProperties.getProperty("versionName")} != $projectVersion"
        }
        check(fdroidVersionProperties.getProperty("versionCode") == androidAppVersionCode.toString()) {
            "F-Droid versionCode must match computed Android versionCode: ${fdroidVersionProperties.getProperty("versionCode")} != $androidAppVersionCode"
        }
        check(fdroidDataMetadataFile.isFile) { "Missing fdroiddata metadata draft: $fdroidDataMetadataFile" }
        check(fdroidMetadataSmokeScript.isFile) { "Missing F-Droid metadata smoke script: $fdroidMetadataSmokeScript" }
        check(fdroidReleaseActivationSmokeScript.isFile) {
            "Missing F-Droid release activation smoke script: $fdroidReleaseActivationSmokeScript"
        }
        check(githubSigningSecretsSmokeScript.isFile) {
            "Missing GitHub signing secrets preflight script: $githubSigningSecretsSmokeScript"
        }
        val fdroidDataMetadata = fdroidDataMetadataFile.readText()
        val requiredFdroidDataSnippets = listOf(
            "RepoType: git",
            "Repo: https://github.com/ai-cli-terminal/terminal.git",
            "versionName: $projectVersion",
            "versionCode: $androidAppVersionCode",
            "disable: Pending next Android release tag that includes F-Droid metadata and fdroid-version.properties",
            "commit: TODO_NEXT_ANDROID_RELEASE_COMMIT",
            "subdir: android",
            "ndk: 28.2.13676358",
            "ANDROID_NDK_HOME=\"\$\$NDK\$\$\" ./build-rust-jni.sh --profile release --no-rustup-target-install",
            "./gradlew :app:verifyFdroidReleaseInputs :app:assembleRelease :app:verifyNativeLibraries",
            "output: app/build/outputs/apk/release/app-release-unsigned.apk",
            "AutoUpdateMode: Version v%v",
            "UpdateCheckData: android/fdroid-version.properties",
            "CurrentVersion: $projectVersion",
            "CurrentVersionCode: $androidAppVersionCode",
        )
        requiredFdroidDataSnippets.forEach { snippet ->
            check(fdroidDataMetadata.contains(snippet)) {
                "fdroiddata metadata draft is missing expected content: $snippet"
            }
        }
        val screenshots = screenshotDir.asFile
            .takeIf { it.isDirectory }
            ?.listFiles { file -> file.isFile && file.extension.equals("png", ignoreCase = true) }
            ?.toList()
            .orEmpty()
        check(screenshots.size >= minimumScreenshotCount) {
            "Expected at least $minimumScreenshotCount Android phone screenshot PNG(s) in ${screenshotDir.asFile}"
        }
        screenshots.forEach { file ->
            check(file.length() > 0) { "Android phone screenshot is empty: $file" }
        }
        check(!releaseSigningConfigured || !releaseKeystorePath.isNullOrBlank()) {
            "Release signing is partially configured"
        }
    }
}
