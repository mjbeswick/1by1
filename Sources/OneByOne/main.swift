import AppKit

if CommandLine.arguments.contains("--selftest") {
    exit(runLogicSelfTest() ? 0 : 1)
}
