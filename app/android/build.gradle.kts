allprojects {
    repositories {
        google()
        mavenCentral()
    }
}

val newBuildDir: Directory =
    rootProject.layout.buildDirectory
        .dir("../../build")
        .get()
rootProject.layout.buildDirectory.value(newBuildDir)

subprojects {
    val newSubprojectBuildDir: Directory = newBuildDir.dir(project.name)
    project.layout.buildDirectory.value(newSubprojectBuildDir)
}

// Force a single NDK version and compile SDK for all Android subprojects.
// Plugins such as `jni` request their own versions, and the Nix SDK is
// read-only, so Gradle can't download anything that isn't already installed.
// Keep in sync with app/build.gradle.kts.
subprojects {
    afterEvaluate {
        extensions.findByType(com.android.build.gradle.BaseExtension::class.java)
            ?.apply {
                ndkVersion = "29.0.14206865"
                if (project.name != "app") compileSdkVersion(36)
            }
    }
}

subprojects {
    project.evaluationDependsOn(":app")
}

tasks.register<Delete>("clean") {
    delete(rootProject.layout.buildDirectory)
}