import Foundation

/// Repeat behaviour of the play queue.
enum Repeat: String, Codable, CaseIterable {
    case off, all, one

    var next: Repeat {
        switch self {
        case .off: return .all
        case .all: return .one
        case .one: return .off
        }
    }

    var label: String {
        switch self {
        case .off: return "Off"
        case .all: return "All"
        case .one: return "Track"
        }
    }
}

/// Index after `cur`. `nil` when play stops.
func nextIndex(count: Int, cur: Int, repeat rep: Repeat) -> Int? {
    if count == 0 { return nil }
    switch rep {
    case .one: return cur
    default:
        if cur + 1 < count { return cur + 1 }
        return rep == .all ? 0 : nil
    }
}

/// Index before `cur`. Wraps to the end with repeat-all.
func prevIndex(count: Int, cur: Int, repeat rep: Repeat) -> Int {
    if count == 0 { return 0 }
    if cur == 0 { return rep == .all ? count - 1 : 0 }
    return cur - 1
}

/// Walk forward from `cur` until `tryPlay` accepts one. Stops after one full pass.
func advancePlayable(count: Int, cur: Int, repeat rep: Repeat, _ tryPlay: (Int) -> Bool) -> Int? {
    var at = cur
    for _ in 0..<count {
        guard let next = nextIndex(count: count, cur: at, repeat: rep) else { return nil }
        if tryPlay(next) { return next }
        at = next
    }
    return nil
}

/// Fisher-Yates with xorshift, as the Rust build did.
func shuffled<T>(_ items: [T], seed: UInt64) -> [T] {
    var out = items
    var s = seed | 1
    guard out.count > 1 else { return out }
    for i in stride(from: out.count - 1, through: 1, by: -1) {
        s ^= s << 13
        s ^= s >> 7
        s ^= s << 17
        let j = Int(s % UInt64(i + 1))
        out.swapAt(i, j)
    }
    return out
}

/// `m:ss` for short values, `h:mm:ss` past an hour.
/// Minutes as `m:ss` (true) or `m'ss` (false) below an hour. Set from settings.
var clockColonMinutes = true

func clock(_ seconds: Double) -> String {
    let s = Int(max(0, seconds))
    if s >= 3600 {
        return String(format: "%d:%02d:%02d", s / 3600, (s % 3600) / 60, s % 60)
    }
    if !clockColonMinutes { return String(format: "%d'%02d", s / 60, s % 60) }
    return String(format: "%d:%02d", s / 60, s % 60)
}

/// Natural order: digit runs compare by value, case is ignored.
func naturalCompare(_ a: String, _ b: String) -> ComparisonResult {
    let x = Array(a.unicodeScalars), y = Array(b.unicodeScalars)
    var i = 0, j = 0
    func isDigit(_ c: Unicode.Scalar) -> Bool { c.value >= 48 && c.value <= 57 }
    while i < x.count && j < y.count {
        let ca = x[i], cb = y[j]
        if isDigit(ca) && isDigit(cb) {
            let si = i, sj = j
            while i < x.count && isDigit(x[i]) { i += 1 }
            while j < y.count && isDigit(y[j]) { j += 1 }
            let va = String(String.UnicodeScalarView(x[si..<i])).drop { $0 == "0" }
            let vb = String(String.UnicodeScalarView(y[sj..<j])).drop { $0 == "0" }
            if va.count != vb.count { return va.count < vb.count ? .orderedAscending : .orderedDescending }
            if va != vb { return va < vb ? .orderedAscending : .orderedDescending }
        } else {
            let la = lower(ca), lb = lower(cb)
            if la != lb { return la < lb ? .orderedAscending : .orderedDescending }
            i += 1
            j += 1
        }
    }
    let rest = (x.count - i) - (y.count - j)
    return rest == 0 ? .orderedSame : (rest < 0 ? .orderedAscending : .orderedDescending)
}

private func lower(_ c: Unicode.Scalar) -> UInt32 {
    (c.value >= 65 && c.value <= 90) ? c.value + 32 : c.value
}

/// Case-insensitive substring test used by Find in list.
func containsCI(_ hay: String, _ needle: String) -> Bool {
    needle.isEmpty || hay.localizedCaseInsensitiveContains(needle)
}

/// Bytes as `123 KB` or `1.23 MB`, as the list shows them.
func sizeText(_ bytes: UInt64) -> String {
    let b = Double(bytes)
    if b >= 1024 * 1024 { return String(format: "%.2f MB", b / (1024 * 1024)) }
    return "\(Int((b / 1024).rounded())) KB"
}

/// Days since 1970-01-01 to civil date. Howard Hinnant's algorithm.
func civilFromDays(_ z0: Int) -> (year: Int, month: Int, day: Int) {
    let z = z0 + 719_468
    let era = (z >= 0 ? z : z - 146_096) / 146_097
    let doe = z - era * 146_097
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365
    let y = yoe + era * 400
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100)
    let mp = (5 * doy + 2) / 153
    let d = doy - (153 * mp + 2) / 5 + 1
    let m = mp < 10 ? mp + 3 : mp - 9
    return (m <= 2 ? y + 1 : y, m, d)
}

/// UTC `YYYY-MM-DD` for a date.
func dateText(_ t: Date) -> String {
    let days = Int(floor(t.timeIntervalSince1970 / 86_400))
    let (y, m, d) = civilFromDays(days)
    return String(format: "%04d-%02d-%02d", y, m, d)
}

/// UTC `YYYY-MM-DD HH:MM`.
func dateTimeText(_ t: Date) -> String {
    let secs = Int(floor(t.timeIntervalSince1970))
    let h = (secs / 3600) % 24, mi = (secs / 60) % 60
    return "\(dateText(t)) \(String(format: "%02d:%02d", h, mi))"
}

/// Runnable checks, invoked by `OneByOne --selftest`. Exits non-zero on failure.
func runLogicSelfTest() -> Bool {
    var ok = true
    func check(_ cond: Bool, _ what: String) {
        if !cond { print("FAIL \(what)"); ok = false }
    }
    check(nextIndex(count: 3, cur: 0, repeat: .off) == 1, "next off")
    check(nextIndex(count: 3, cur: 2, repeat: .off) == nil, "next end off")
    check(nextIndex(count: 3, cur: 2, repeat: .all) == 0, "next wrap all")
    check(nextIndex(count: 3, cur: 1, repeat: .one) == 1, "next one")
    check(nextIndex(count: 0, cur: 0, repeat: .all) == nil, "next empty")
    check(prevIndex(count: 3, cur: 0, repeat: .all) == 2, "prev wrap")
    check(prevIndex(count: 3, cur: 0, repeat: .off) == 0, "prev off")
    check(Repeat.off.next.next.next == .off, "repeat cycle")
    check(advancePlayable(count: 3, cur: 0, repeat: .off, { [false, true, true][$0] == false }) == nil, "skip all bad")
    check(advancePlayable(count: 3, cur: 0, repeat: .off, { $0 == 2 }) == 2, "skip to good")
    check(advancePlayable(count: 3, cur: 0, repeat: .all, { _ in false }) == nil, "all bad ends")
    check(advancePlayable(count: 2, cur: 1, repeat: .all, { $0 == 0 }) == 0, "wrap good")
    let base = Array(0..<50)
    let sh = shuffled(base, seed: 12345)
    check(sh.sorted() == base && sh != base, "shuffle permutes")
    check(clock(615) == "10:15", "clock mm:ss")
    check(clock(3615) == "1:00:15", "clock h:mm:ss")
    check(naturalCompare("track9.mp3", "track10.mp3") == .orderedAscending, "natural 9<10")
    check(naturalCompare("Track9.mp3", "track9.mp3") == .orderedSame, "natural case")
    check(naturalCompare("a01", "a1") == .orderedSame, "natural zeros")
    check(naturalCompare("b", "a") == .orderedDescending, "natural alpha")
    check(sizeText(2048) == "2 KB", "size KB")
    check(sizeText(3 * 1024 * 1024) == "3.00 MB", "size MB")
    check(dateText(Date(timeIntervalSince1970: 86_400)) == "1970-01-02", "date")
    check(civilFromDays(19_000) == (2022, 1, 8), "civil")
    print(ok ? "selftest logic: ok" : "selftest logic: FAILED")
    return ok
}
