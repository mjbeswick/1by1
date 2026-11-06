import AppKit

let args = Array(CommandLine.arguments.dropFirst())

if args.first == "--selftest" {
    exit(runLogicSelfTest() ? 0 : 1)
}

if args.first == "--smoke" {
    runAudioSmoke(paths: Array(args.dropFirst()))
}
