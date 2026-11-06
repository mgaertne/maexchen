import net.ltgt.gradle.errorprone.errorprone

plugins {
    application
    jacoco
    id("com.diffplug.spotless") version "8.11.0"
    id("net.ltgt.errorprone") version "5.2.0"
}

repositories {
    mavenCentral()
}

dependencies {
    errorprone(libs.errorprone.core)
    testImplementation(platform(libs.junit.bom))
    testImplementation(libs.junit.jupiter)
    testRuntimeOnly(libs.junit.platform.launcher)
}

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(21)
    }
}

application {
    mainClass = "maexchen.Main"
}

spotless {
    java {
        googleJavaFormat()
    }
}

tasks.withType<JavaCompile> {
    options.release = 21
    options.compilerArgs.addAll(listOf("-Xlint:all,-serial", "-Werror"))
    options.errorprone {
        allSuggestionsAsWarnings = true
        enable("UnnecessarilyFullyQualified", "CheckedExceptionNotThrown", "PrivateConstructorForUtilityClass")
    }
}

tasks.test {
    useJUnitPlatform()
    finalizedBy(tasks.jacocoTestReport)
}

tasks.jacocoTestReport {
    reports {
        xml.required = true
    }
}

tasks.jacocoTestCoverageVerification {
    violationRules {
        rule {
            limit {
                minimum = "0.80".toBigDecimal()
            }
        }
    }
}

tasks.check {
    dependsOn(tasks.jacocoTestCoverageVerification)
}
